use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use super::assemble::{
    execute_audio_only_export, execute_full_reencode_export,
    execute_individual_clips_export, execute_smartcut_export_with_options,
};
use super::compatibility::validate_container_codec_compatibility;
use super::packets::get_keyframes_with_gop_info;
use super::planner::{create_full_export_plan, normalize_segments, FullExportPlan, NormalizedSegment};
use super::probe::{probe_source_info, SourceInfo};
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
    pub plan_id: Option<String>,
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
    pub plan_id: Option<String>,
    pub total_duration: f64,
    pub reencode_duration: f64,
    pub copy_duration: f64,
    pub segments_count: usize,
    pub pieces_count: usize,
}

#[derive(Clone)]
pub struct CachedPlanEntry {
    pub plan_id: String,
    pub input_hash: u64,
    pub plan: FullExportPlan,
    pub source: SourceInfo,
    pub normalized_segments: Vec<NormalizedSegment>,
    pub created_at: Instant,
}

static PLAN_CACHE: LazyLock<Mutex<HashMap<String, CachedPlanEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn compute_plan_signature(options: &ExportOptions) -> u64 {
    let mut hasher = DefaultHasher::new();
    options.input_path.hash(&mut hasher);
    options.mode.as_deref().unwrap_or("smart_cut").hash(&mut hasher);
    options.audio_only.unwrap_or(false).hash(&mut hasher);
    options.audio_format.as_deref().unwrap_or("").hash(&mut hasher);
    options.container_format.as_deref().unwrap_or("").hash(&mut hasher);
    options.video_codec.as_deref().unwrap_or("").hash(&mut hasher);

    for s in &options.segments {
        ((s.start * 1000.0) as i64).hash(&mut hasher);
        ((s.end * 1000.0) as i64).hash(&mut hasher);
        s.name.as_deref().unwrap_or("").hash(&mut hasher);
    }

    if let Ok(meta) = Path::new(&options.input_path).metadata() {
        meta.len().hash(&mut hasher);
        if let Ok(mtime) = meta.modified() {
            if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                dur.as_secs().hash(&mut hasher);
            }
        }
    }

    hasher.finish()
}

#[tauri::command]
pub async fn get_export_plan_preview(options: ExportOptions) -> Result<ExportPlanSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = probe_source_info(&options.input_path)?;
        let audio_only = options.audio_only.unwrap_or(false);

        // VF-11: Validar existencia de pista de video en exportaciones de video
        if !audio_only && source.video.is_none() {
            return Err("El archivo de entrada no contiene pista de video. Para procesar este archivo, active la opción 'Extraer solo audio'.".to_string());
        }

        let mode = options.mode.as_deref().unwrap_or("smart_cut");
        let container = options
            .container_format
            .as_deref()
            .unwrap_or(&source.container_ext);

        // VF-08: Validar compatibilidad de contenedor y codec
        validate_container_codec_compatibility(
            container,
            &source,
            mode,
            options.video_codec.as_deref(),
            audio_only,
        )?;

        let normalized = normalize_segments(&options.segments, source.duration);
        let total_dur: f64 = normalized.iter().map(|s| (s.end - s.start).max(0.0)).sum();

        if normalized.is_empty() {
            return Ok(ExportPlanSummary {
                plan_id: None,
                total_duration: 0.0,
                reencode_duration: 0.0,
                copy_duration: 0.0,
                segments_count: 0,
                pieces_count: 0,
            });
        }

        // VF-09: Identificador único y caché del plan preview
        let input_hash = compute_plan_signature(&options);
        let plan_id = format!("plan_{:016x}", input_hash);

        // Si ya está en caché y es reciente (< 5 min), reutilizar
        {
            let cache = PLAN_CACHE.lock().unwrap();
            if let Some(entry) = cache.get(&plan_id) {
                if entry.input_hash == input_hash && entry.created_at.elapsed().as_secs() < 300 {
                    let mut total_pieces = 0;
                    for s in &entry.plan.segments {
                        total_pieces += s.video_pieces.len();
                    }
                    return Ok(ExportPlanSummary {
                        plan_id: Some(plan_id),
                        total_duration: entry.plan.total_export_duration,
                        reencode_duration: entry.plan.total_reencode_duration,
                        copy_duration: entry.plan.total_copy_duration,
                        segments_count: entry.plan.segments.len(),
                        pieces_count: total_pieces,
                    });
                }
            }
        }

        if audio_only {
            return Ok(ExportPlanSummary {
                plan_id: Some(plan_id),
                total_duration: total_dur,
                reencode_duration: 0.0,
                copy_duration: total_dur,
                segments_count: normalized.len(),
                pieces_count: normalized.len(),
            });
        }

        if mode == "reencode" {
            return Ok(ExportPlanSummary {
                plan_id: Some(plan_id),
                total_duration: total_dur,
                reencode_duration: total_dur,
                copy_duration: 0.0,
                segments_count: normalized.len(),
                pieces_count: normalized.len(),
            });
        }

        let video_ordinal = source.video.as_ref().map(|v| v.video_index).unwrap_or(0);
        let min_start = normalized.iter().map(|s| s.start).fold(f64::INFINITY, f64::min).max(0.0).min(source.duration);
        let max_end = normalized.iter().map(|s| s.end).fold(0.0_f64, f64::max).max(0.0).min(source.duration);

        let keyframes = get_keyframes_with_gop_info(&options.input_path, video_ordinal, min_start, max_end)
            .unwrap_or_default();

        let plan = create_full_export_plan(&source, &normalized, &keyframes);
        let mut total_pieces = 0;
        for s in &plan.segments {
            total_pieces += s.video_pieces.len();
        }

        let summary = ExportPlanSummary {
            plan_id: Some(plan_id.clone()),
            total_duration: plan.total_export_duration,
            reencode_duration: plan.total_reencode_duration,
            copy_duration: plan.total_copy_duration,
            segments_count: plan.segments.len(),
            pieces_count: total_pieces,
        };

        {
            let mut cache = PLAN_CACHE.lock().unwrap();
            cache.insert(
                plan_id.clone(),
                CachedPlanEntry {
                    plan_id,
                    input_hash,
                    plan,
                    source,
                    normalized_segments: normalized,
                    created_at: Instant::now(),
                },
            );
        }

        Ok(summary)
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

        let out_p = Path::new(output_path);
        let ext = out_p
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .or_else(|| options.container_format.clone())
            .unwrap_or_else(|| "mp4".to_string());

        let audio_only = options.audio_only.unwrap_or(false);
        let mode = options.mode.as_deref().unwrap_or("smart_cut");

        // 1. Análisis de stream y formato original
        let source = probe_source_info(input_path)?;

        // VF-11: Validar existencia de pista de video en exportaciones de video
        if !audio_only && source.video.is_none() {
            return Err("El archivo de entrada no contiene pista de video. Para procesar este archivo, active la opción 'Extraer solo audio'.".to_string());
        }

        // VF-08: Validar compatibilidad de contenedor y codec
        validate_container_codec_compatibility(
            &ext,
            &source,
            mode,
            options.video_codec.as_deref(),
            audio_only,
        )?;

        // 2. Normalización de segmentos
        let normalized = normalize_segments(&options.segments, source.duration);
        if normalized.is_empty() {
            return Err("Todos los segmentos son de duración nula".to_string());
        }

        // VF-09: Validar si existe un plan previewed coincidente
        let current_hash = compute_plan_signature(&options);
        let cached_entry = options.plan_id.as_ref().and_then(|pid| {
            let cache = PLAN_CACHE.lock().unwrap();
            cache.get(pid).filter(|e| e.input_hash == current_hash).cloned()
        });

        // 3. Directorio temporal con limpieza garantizada (RAII Drop)
        let temp_dir = TempDir::new("export")?;

        // 4. Despachar según el modo seleccionado (VF-01 y VF-02)
        let result_path = if audio_only {
            if mode == "individual_clips" {
                execute_individual_clips_export(
                    &source,
                    &normalized,
                    output_path,
                    true,
                    options.audio_format.as_deref(),
                    options.hardware_accel.as_deref(),
                    options.video_codec.as_deref(),
                    options.crf,
                    options.preset.as_deref(),
                    &temp_dir,
                    Some(&app_handle),
                )?
            } else {
                execute_audio_only_export(
                    &source,
                    &normalized,
                    output_path,
                    options.audio_format.as_deref(),
                    &temp_dir,
                    Some(&app_handle),
                )?
            }
        } else {
            match mode {
                "individual_clips" => {
                    execute_individual_clips_export(
                        &source,
                        &normalized,
                        output_path,
                        false,
                        None,
                        options.hardware_accel.as_deref(),
                        options.video_codec.as_deref(),
                        options.crf,
                        options.preset.as_deref(),
                        &temp_dir,
                        Some(&app_handle),
                    )?
                }
                "reencode" => {
                    execute_full_reencode_export(
                        &source,
                        &normalized,
                        output_path,
                        options.hardware_accel.as_deref(),
                        options.video_codec.as_deref(),
                        options.crf,
                        options.preset.as_deref(),
                        &temp_dir,
                        Some(&app_handle),
                    )?
                }
                _ => {
                    // Smart Cut (por defecto)
                    if source.video.is_none() {
                        return Err("El archivo no contiene pista de video para exportar en modo Smart Cut".to_string());
                    }

                    // VF-09: Usar el plan previewed idéntico si coincide con el plan_id y firma de opciones
                    let plan = if let Some(ref entry) = cached_entry {
                        entry.plan.clone()
                    } else {
                        let video_ordinal = source.video.as_ref().map(|v| v.video_index).unwrap_or(0);
                        let min_start = normalized.iter().map(|s| s.start).fold(f64::INFINITY, f64::min).max(0.0).min(source.duration);
                        let max_end = normalized.iter().map(|s| s.end).fold(0.0_f64, f64::max).max(0.0).min(source.duration);

                        let keyframes = get_keyframes_with_gop_info(input_path, video_ordinal, min_start, max_end)
                            .unwrap_or_default();

                        create_full_export_plan(&source, &normalized, &keyframes)
                    };

                    execute_smartcut_export_with_options(
                        &source,
                        &plan,
                        output_path,
                        &temp_dir,
                        options.hardware_accel.as_deref(),
                        options.crf,
                        options.preset.as_deref(),
                        options.video_codec.as_deref(),
                        Some(&app_handle),
                    )?
                }
            }
        };

        // 5. Notificar 100% completado
        let final_progress = ExportProgress {
            percent: 100.0,
            stage: "¡Exportación completada exitosamente!".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vf08_container_validation_in_plan_preview() {
        tauri::async_runtime::block_on(async {
            let path = "RECURSOS/videoprueba.mp4";
            if !Path::new(path).exists() {
                return;
            }

            // WebM no es compatible con video H.264 en smart_cut
            let opts_incompatible = ExportOptions {
                input_path: path.to_string(),
                output_path: "out.webm".to_string(),
                plan_id: None,
                mode: Some("smart_cut".to_string()),
                segments: vec![ExportSegment { start: 1.0, end: 3.0, name: None }],
                hardware_accel: None,
                video_codec: None,
                crf: None,
                preset: None,
                audio_only: Some(false),
                audio_format: None,
                container_format: Some("webm".to_string()),
            };

            let res = get_export_plan_preview(opts_incompatible).await;
            assert!(res.is_err(), "WebM with H.264 in smart_cut must fail validation");
            assert!(res.unwrap_err().contains("El contenedor WebM solo es compatible"));

            // MKV sí es compatible universalmente
            let opts_compatible = ExportOptions {
                input_path: path.to_string(),
                output_path: "out.mkv".to_string(),
                plan_id: None,
                mode: Some("smart_cut".to_string()),
                segments: vec![ExportSegment { start: 1.0, end: 3.0, name: None }],
                hardware_accel: None,
                video_codec: None,
                crf: None,
                preset: None,
                audio_only: Some(false),
                audio_format: None,
                container_format: Some("mkv".to_string()),
            };

            let res_ok = get_export_plan_preview(opts_compatible).await;
            assert!(res_ok.is_ok(), "MKV with H.264 must succeed: {:?}", res_ok.err());
        });
    }

    #[test]
    fn test_vf09_plan_id_preview_and_caching() {
        tauri::async_runtime::block_on(async {
            let path = "RECURSOS/videoprueba.mp4";
            if !Path::new(path).exists() {
                return;
            }

            let opts1 = ExportOptions {
                input_path: path.to_string(),
                output_path: "out1.mp4".to_string(),
                plan_id: None,
                mode: Some("smart_cut".to_string()),
                segments: vec![ExportSegment { start: 1.0, end: 3.0, name: None }],
                hardware_accel: None,
                video_codec: None,
                crf: None,
                preset: None,
                audio_only: Some(false),
                audio_format: None,
                container_format: Some("mp4".to_string()),
            };

            let preview1 = get_export_plan_preview(opts1.clone()).await.expect("preview failed");
            assert!(preview1.plan_id.is_some(), "VF-09: Preview must return plan_id");
            let pid1 = preview1.plan_id.unwrap();

            // 1. Verificar que el plan existe en PLAN_CACHE
            {
                let cache = PLAN_CACHE.lock().unwrap();
                assert!(cache.contains_key(&pid1), "PLAN_CACHE must contain plan_id");
            }

            // 2. Re-consultar con las mismas opciones debe devolver el mismo plan_id desde caché
            let preview2 = get_export_plan_preview(opts1.clone()).await.expect("preview2 failed");
            assert_eq!(preview2.plan_id.as_deref(), Some(pid1.as_str()), "Identical options must return same plan_id");

            // 3. Modificar segmentos debe generar un plan_id diferente
            let mut opts_diff = opts1.clone();
            opts_diff.segments = vec![ExportSegment { start: 2.0, end: 5.0, name: None }];
            let preview_diff = get_export_plan_preview(opts_diff).await.expect("preview_diff failed");
            assert_ne!(preview_diff.plan_id.as_deref(), Some(pid1.as_str()), "Modified options must produce distinct plan_id");
        });
    }

    #[test]
    fn test_vf11_missing_video_track_validation() {
        tauri::async_runtime::block_on(async {
            let temp_dir = std::env::temp_dir().join(format!("smartcut_test_vf11_{}", std::process::id()));
            let _ = std::fs::create_dir_all(&temp_dir);
            let audio_file = temp_dir.join("test_pure_audio.mp3");

            // Generar archivo de audio puro con ffmpeg
            let status = std::process::Command::new("ffmpeg")
                .args([
                    "-y",
                    "-f", "lavfi",
                    "-i", "anullsrc=r=44100:cl=mono",
                    "-t", "1.5",
                    "-c:a", "libmp3lame",
                    &audio_file.to_string_lossy(),
                ])
                .output();

            if status.is_err() || !status.unwrap().status.success() {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return;
            }

            // 1. Intentar previsualizar plan como video (audio_only: false) debe fallar con VF-11
            let opts_video = ExportOptions {
                input_path: audio_file.to_string_lossy().to_string(),
                output_path: "out.mp4".to_string(),
                plan_id: None,
                mode: Some("smart_cut".to_string()),
                segments: vec![ExportSegment { start: 0.0, end: 1.0, name: None }],
                hardware_accel: None,
                video_codec: None,
                crf: None,
                preset: None,
                audio_only: Some(false),
                audio_format: None,
                container_format: Some("mp4".to_string()),
            };

            let res_video = get_export_plan_preview(opts_video).await;
            assert!(res_video.is_err(), "Video export of audio-only source must fail");
            assert!(
                res_video.unwrap_err().contains("El archivo de entrada no contiene pista de video"),
                "Error must clearly guide user to extract audio only"
            );

            // 2. Con audio_only: true debe proceder exitosamente
            let opts_audio = ExportOptions {
                input_path: audio_file.to_string_lossy().to_string(),
                output_path: "out.mp3".to_string(),
                plan_id: None,
                mode: Some("smart_cut".to_string()),
                segments: vec![ExportSegment { start: 0.0, end: 1.0, name: None }],
                hardware_accel: None,
                video_codec: None,
                crf: None,
                preset: None,
                audio_only: Some(true),
                audio_format: Some("mp3".to_string()),
                container_format: None,
            };

            let res_audio = get_export_plan_preview(opts_audio).await;
            assert!(res_audio.is_ok(), "Audio-only export of audio source must succeed: {:?}", res_audio.err());

            let _ = std::fs::remove_dir_all(&temp_dir);
        });
    }
}

