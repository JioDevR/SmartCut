pub mod export;
pub mod ffmpeg;
pub mod server;

#[tauri::command]
async fn pick_video_file() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let file = rfd::FileDialog::new()
            .set_title("Seleccionar archivo de video (SmartCut)")
            .add_filter(
                "Videos soportados (*.mp4, *.mkv, *.mov, *.webm)",
                &["mp4", "mkv", "mov", "webm"],
            )
            .pick_file();

        Ok(file.map(|p| p.to_string_lossy().to_string()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn save_project_dialog(content: String, default_name: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let name = default_name.unwrap_or_else(|| "proyecto.smartcut".to_string());
        let dialog = rfd::FileDialog::new()
            .set_title("Guardar sesión de proyecto (SmartCut)")
            .set_file_name(&name)
            .add_filter("Proyecto SmartCut (*.smartcut)", &["smartcut"]);

        if let Some(path) = dialog.save_file() {
            std::fs::write(&path, content).map_err(|e| format!("Error guardando proyecto: {}", e))?;
            Ok(Some(path.to_string_lossy().to_string()))
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn open_project_dialog() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dialog = rfd::FileDialog::new()
            .set_title("Abrir proyecto SmartCut")
            .add_filter("Proyecto SmartCut (*.smartcut)", &["smartcut"]);

        if let Some(path) = dialog.pick_file() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Error leyendo archivo de proyecto: {}", e))?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn save_edl_dialog(content: String, default_name: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let name = default_name.unwrap_or_else(|| "timeline.edl".to_string());
        let dialog = rfd::FileDialog::new()
            .set_title("Exportar EDL (CMX 3600)")
            .set_file_name(&name)
            .add_filter("Lista de Decisión de Edición (*.edl)", &["edl"]);

        if let Some(path) = dialog.save_file() {
            std::fs::write(&path, content).map_err(|e| format!("Error guardando EDL: {}", e))?;
            Ok(Some(path.to_string_lossy().to_string()))
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    server::start_streaming_server();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            pick_video_file,
            ffmpeg::get_video_metadata,
            ffmpeg::get_sample_videos,
            server::get_video_stream_url,
            ffmpeg::get_timeline_thumbnails,
            ffmpeg::get_thumbnails_for_range,
            ffmpeg::get_audio_visualization,
            export::get_available_encoders,
            export::pick_save_path,
            export::export_video,
            export::cancel_export,
            export::generate_compatible_preview,
            export::get_export_plan_preview,
            save_project_dialog,
            open_project_dialog,
            save_edl_dialog
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
