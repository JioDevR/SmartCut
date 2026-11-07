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

#[tauri::command]
pub async fn get_audio_peaks(path: String) -> Result<Vec<u8>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        extract_audio_peaks(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn extract_audio_peaks(path: &str) -> Result<Vec<u8>, String> {
    let p = Path::new(path);
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

    let temp_dir = std::env::temp_dir();
    let cache_file = temp_dir.join(format!("smartcut_peaks_{:016x}.bin", path_hash));

    if cache_file.exists() {
        if let Ok(bytes) = std::fs::read(&cache_file) {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
    }

    // Ejecutar ffmpeg para extraer audio PCM s16le a 8000 Hz mono a través de stdout pipe
    let mut child = match Command::new("ffmpeg")
        .args([
            "-i",
            path,
            "-vn",
            "-sn",
            "-dn",
            "-ac",
            "1",
            "-ar",
            "8000",
            "-f",
            "s16le",
            "-threads",
            "2",
            "-v",
            "error",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Err(format!("Error iniciando ffmpeg: {}", e)),
    };

    let mut stdout = match child.stdout.take() {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };

    // 8000 Hz mono s16le: 1 muestra = 2 bytes.
    // A 100 picos por segundo: 1 pico cada 80 muestras = 160 bytes (ventana de 10ms por pico).
    let samples_per_window = 80;
    let bytes_per_window = samples_per_window * 2;
    let mut window_buf = vec![0u8; bytes_per_window];
    let mut peaks = Vec::new();

    use std::io::Read;
    loop {
        let mut read_bytes = 0;
        while read_bytes < bytes_per_window {
            match stdout.read(&mut window_buf[read_bytes..]) {
                Ok(0) => break, // EOF
                Ok(n) => read_bytes += n,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }

        if read_bytes == 0 {
            break;
        }

        let samples_read = read_bytes / 2;
        let mut max_abs: i16 = 0;
        for i in 0..samples_read {
            let sample = i16::from_le_bytes([window_buf[i * 2], window_buf[i * 2 + 1]]);
            let abs = if sample == i16::MIN {
                i16::MAX
            } else {
                sample.abs()
            };
            if abs > max_abs {
                max_abs = abs;
            }
        }

        // Normalización perceptual no lineal (compresión de rango dinámico para voces)
        // (max_abs / 32767.0)^0.7 * 255.0
        let normalized = (max_abs as f32 / 32767.0).powf(0.7).clamp(0.0, 1.0);
        let peak_val = (normalized * 255.0).round() as u8;
        peaks.push(peak_val);
    }

    let _ = child.wait();

    if !peaks.is_empty() {
        let _ = std::fs::write(&cache_file, &peaks);
    }

    Ok(peaks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_audio_peaks() {
        let temp_dir = std::env::temp_dir();
        let test_audio = temp_dir.join(format!("smartcut_test_sine_{}.wav", std::process::id()));

        // Generar un archivo wav de 2 segundos a 1000 Hz con ffmpeg
        let gen_status = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:duration=2",
                "-c:a",
                "pcm_s16le",
                test_audio.to_str().unwrap(),
            ])
            .output();

        if let Ok(out) = gen_status {
            if out.status.success() {
                let peaks = extract_audio_peaks(test_audio.to_str().unwrap()).unwrap();
                // 2 segundos a 100 picos/seg = ~200 picos (+/- margen de decodificación)
                assert!(peaks.len() >= 180 && peaks.len() <= 220);
                // Con tono senoidal a -18dB (amplitud 1/8), el pico normalizado es ~59
                assert!(peaks.iter().any(|&p| p > 40));
            }
        }

        let _ = std::fs::remove_file(test_audio);
    }
}
