use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, Emitter};

use super::assemble::execute_smartcut_export;
use super::packets::get_keyframes_with_gop_info;
use super::planner::{create_full_export_plan, normalize_segments};
use super::probe::probe_source_info;
use super::runner::{cancel_current_export, reset_cancel_state, ExportProgress};
use super::temp::TempDir;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportSegment {
    pub start: f64,
    pub end: f64,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportOptions {
    pub input_path: String,
    pub output_path: String,
    pub mode: Option<String>,
    pub segments: Vec<ExportSegment>,
    pub hardware_accel: Option<String>,
    pub video_codec: Option<String>,
    pub crf: Option<u32>,
    pub preset: Option<String>,
    pub audio_only: Option<bool>,
    pub audio_format: Option<String>,
    pub container_format: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportPlanSummary {
    pub total_duration: f64,
    pub reencode_duration: f64,
    pub copy_duration: f64,
    pub segments_count: usize,
    pub pieces_count: usize,
}

#[tauri::command]
pub async fn get_export_plan_preview(options: ExportOptions) -> Result<ExportPlanSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = probe_source_info(&options.input_path)?;
        let normalized = normalize_segments(&options.segments, source.duration);
        let video_idx = source.video.as_ref().map(|v| v.stream_index).unwrap_or(0);

        let min_start = normalized.first().map(|s| s.start).unwrap_or(0.0);
        let max_end = normalized.last().map(|s| s.end).unwrap_or(source.duration);

        let keyframes = get_keyframes_with_gop_info(&options.input_path, video_idx, min_start, max_end)
            .unwrap_or_default();

        let plan = create_full_export_plan(&source, &normalized, &keyframes);
        let mut total_pieces = 0;
        for s in &plan.segments {
            total_pieces += s.video_pieces.len();
        }

        Ok(ExportPlanSummary {
            total_duration: plan.total_export_duration,
            reencode_duration: plan.total_reencode_duration,
            copy_duration: plan.total_copy_duration,
            segments_count: plan.segments.len(),
            pieces_count: total_pieces,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_video(options: ExportOptions, app_handle: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        reset_cancel_state();

        let input_path = &options.input_path;
        let output_path = &options.output_path;

        if options.segments.is_empty() {
            return Err("No hay clips seleccionados para exportar".to_string());
        }

        let p = Path::new(input_path);
        if !p.exists() {
            return Err(format!("El archivo de entrada no existe: {}", input_path));
        }

        // 1. Análisis de stream y formato original
        let source = probe_source_info(input_path)?;

        // 2. Normalización de segmentos
        let normalized = normalize_segments(&options.segments, source.duration);
        if normalized.is_empty() {
            return Err("Todos los segmentos son de duración nula".to_string());
        }

        // 3. Inspección de paquetes y keyframes en los rangos de corte
        let video_idx = source.video.as_ref().map(|v| v.stream_index).unwrap_or(0);
        let min_start = normalized.first().map(|s| s.start).unwrap_or(0.0);
        let max_end = normalized.last().map(|s| s.end).unwrap_or(source.duration);

        let keyframes = get_keyframes_with_gop_info(input_path, video_idx, min_start, max_end)
            .unwrap_or_default();

        // 4. Creación del plan Smart Cut (función pura)
        let plan = create_full_export_plan(&source, &normalized, &keyframes);

        // 5. Directorio temporal con limpieza garantizada (RAII Drop)
        let temp_dir = TempDir::new("export")?;

        // 6. Ensamblado y ejecución
        let result_path = execute_smartcut_export(
            &source,
            &plan,
            output_path,
            &temp_dir,
            Some(&app_handle),
        )?;

        // 7. Notificar 100% completado
        let final_progress = ExportProgress {
            percent: 100.0,
            stage: "¡Exportación Smart Cut completada exitosamente!".to_string(),
            current_clip: normalized.len(),
            total_clips: normalized.len(),
            fps: 0.0,
            speed: "1.0x".to_string(),
        };
        let _ = app_handle.emit("export-progress", &final_progress);

        Ok(result_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn cancel_export() -> Result<(), String> {
    cancel_current_export();
    Ok(())
}

#[tauri::command]
pub async fn pick_save_path(default_name: Option<String>, default_ext: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let ext = default_ext.unwrap_or_else(|| "mp4".to_string());
        let name = default_name.unwrap_or_else(|| format!("video_smartcut.{}", ext));
        let dialog = rfd::FileDialog::new()
            .set_title("Guardar video exportado (SmartCut)")
            .set_file_name(&name)
            .add_filter(&format!("Formato original (*.{})", ext), &[&ext])
            .add_filter("Todos los formatos de video (*.mp4, *.mkv, *.mov, *.webm)", &["mp4", "mkv", "mov", "webm"]);

        let file = dialog.save_file();
        Ok(file.map(|p| p.to_string_lossy().to_string()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn generate_compatible_preview(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = Path::new(&path);
        if !p.exists() {
            return Err("El archivo no existe".to_string());
        }

        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
        let temp_dir = std::env::temp_dir();

        let meta = p.metadata().ok();
        let file_size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        file_size.hash(&mut hasher);
        mtime.hash(&mut hasher);
        let path_hash = hasher.finish();

        let preview_path = temp_dir.join(format!("smartcut_preview_{}_{:016x}.mp4", stem, path_hash));

        if preview_path.exists() && preview_path.metadata().map(|m| m.len() > 1000).unwrap_or(false) {
            return Ok(preview_path.to_string_lossy().to_string());
        }

        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-y",
            "-i",
            &path,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "26",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-movflags",
            "+faststart",
            preview_path.to_str().unwrap(),
        ]);

        let output = cmd.output().map_err(|e| format!("Error ejecutando ffmpeg: {}", e))?;
        if !output.status.success() {
            let err_str = String::from_utf8_lossy(&output.stderr);
            return Err(format!("FFmpeg falló al generar vista previa: {}", err_str));
        }

        Ok(preview_path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
