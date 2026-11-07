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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    server::start_streaming_server();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            pick_video_file,
            ffmpeg::get_video_metadata,
            server::get_video_stream_url,
            ffmpeg::get_timeline_thumbnails,
            ffmpeg::get_thumbnails_for_range,
            ffmpeg::get_audio_visualization,
            ffmpeg::get_audio_peaks,
            export::get_available_encoders,
            export::pick_save_path,
            export::export_video,
            export::cancel_export,
            export::generate_compatible_preview,
            export::get_export_plan_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
