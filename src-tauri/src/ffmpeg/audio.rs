use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::Ordering;

use crate::server::{get_or_init_session_token, urlencoding_encode, STREAM_SERVER_PORT};

#[tauri::command]
pub async fn get_audio_visualization(path: String, mode: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = Path::new(&path);
        if !p.exists() {
            return Err("Archivo no encontrado".to_string());
        }

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

        let is_spectrogram = mode.as_deref() == Some("spectrogram");
        let prefix = if is_spectrogram { "spectro" } else { "wave" };
        let temp_dir = std::env::temp_dir();
        let audio_file = temp_dir.join(format!("smartcut_audio_{}_{:016x}.png", prefix, path_hash));

        let port = STREAM_SERVER_PORT.load(Ordering::SeqCst);
        let token = get_or_init_session_token();

        if audio_file.exists() && audio_file.metadata().map(|m| m.len() > 100).unwrap_or(false) {
            let encoded_p = urlencoding_encode(&audio_file.to_string_lossy());
            return Ok(format!("http://127.0.0.1:{}/video?token={}&path={}", port, token, encoded_p));
        }

        let lavfi_filter = if is_spectrogram {
            // Alta sensibilidad para espectrograma: escala cúbica/logarítmica y ganancia +5dB
            "aformat=channel_layouts=mono,showspectrumpic=s=1400x56:color=fiery:scale=cbrt:gain=5:legend=0"
        } else {
            // Alta sensibilidad para forma de onda: compresión dinámica para amplificar señales tenues
            "aformat=channel_layouts=mono,compand=attacks=0:points=-80/-80|-40/-10|-20/-2|0/0,showwavespic=s=1400x56:colors=#00f2fe|#4facfe"
        };

        let output = Command::new("ffmpeg")
            .args([
                "-i",
                &path,
                "-vn",
                "-threads",
                "2",
                "-lavfi",
                lavfi_filter,
                "-frames:v",
                "1",
                "-f",
                "image2",
                "-v",
                "error",
                "-y",
                audio_file.to_str().unwrap_or(""),
            ])
            .output()
            .map_err(|e| format!("Error ejecutando ffmpeg: {}", e))?;

        if !output.status.success() || !audio_file.exists() {
            return Err("No se pudo generar forma de onda/espectrograma de audio".to_string());
        }

        let encoded_p = urlencoding_encode(&audio_file.to_string_lossy());
        Ok(format!("http://127.0.0.1:{}/video?token={}&path={}", port, token, encoded_p))
    })
    .await
    .map_err(|e| e.to_string())?
}
