use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyReport {
    pub success: bool,
    pub duration: f64,
    pub file_size: u64,
    pub video_streams: usize,
    pub audio_streams: usize,
}

/// VF-10: Preserva una copia del archivo temporal defectuoso para diagnóstico antes de que se limpie el directorio
fn preserve_temp_failure(path: &str) -> Option<String> {
    let p = Path::new(path);
    if !p.exists() {
        return None;
    }
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("failed");
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("tmp");
    let diag_dir = std::env::temp_dir().join("smartcut_failed_diagnostic");
    let _ = std::fs::create_dir_all(&diag_dir);
    let diag_path = diag_dir.join(format!(
        "{}_{:016x}.{}",
        stem,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        ext
    ));
    if std::fs::copy(p, &diag_path).is_ok() {
        Some(diag_path.to_string_lossy().to_string())
    } else {
        None
    }
}

/// VF-10: Verificación exhaustiva de duración, pistas, decodificación y preservación de diagnósticos
pub fn verify_output_file_full(
    path: &str,
    expected_duration: Option<f64>,
    tolerance_sec: Option<f64>,
    audio_only: bool,
    expected_video_streams: Option<usize>,
    expected_audio_streams: Option<usize>,
) -> Result<VerifyReport, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("El archivo de salida no fue generado: {}", path));
    }

    let file_size = p.metadata().map(|m| m.len()).unwrap_or(0);
    if file_size < 100 {
        let diag = preserve_temp_failure(path);
        let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
        return Err(format!("El archivo de salida está vacío o corrupto (tamaño {} bytes){}", file_size, diag_str));
    }

    // 1. Probar decodificación con FFmpeg sin errores
    let decode_test = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-i",
            path,
            "-f",
            "null",
            "-",
        ])
        .output()
        .map_err(|e| format!("Error verificando archivo con ffmpeg: {}", e))?;

    if !decode_test.status.success() {
        let err = String::from_utf8_lossy(&decode_test.stderr);
        if !err.trim().is_empty() {
            let diag = preserve_temp_failure(path);
            let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
            return Err(format!("Verificación de integridad falló en el archivo generado: {}{}", err, diag_str));
        }
    }

    // 2. Obtener streams presentes en el archivo
    let stream_out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
            path,
        ])
        .output();

    let stream_str = stream_out
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let mut video_streams = 0;
    let mut audio_streams = 0;
    for line in stream_str.lines() {
        let t = line.trim();
        if t == "video" {
            video_streams += 1;
        } else if t == "audio" {
            audio_streams += 1;
        }
    }

    // 3. Validar expectativas exactas de pistas si se especificaron
    if let Some(exp_v) = expected_video_streams {
        if video_streams != exp_v {
            let diag = preserve_temp_failure(path);
            let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
            return Err(format!(
                "Discrepancia en pistas de video: esperadas {}, encontradas {}{}",
                exp_v, video_streams, diag_str
            ));
        }
    }

    if let Some(exp_a) = expected_audio_streams {
        if audio_streams != exp_a {
            let diag = preserve_temp_failure(path);
            let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
            return Err(format!(
                "Discrepancia en pistas de audio: esperadas {}, encontradas {}{}",
                exp_a, audio_streams, diag_str
            ));
        }
    }

    // 4. Validar expectativas de tipo de medio
    if audio_only {
        if video_streams > 0 {
            let diag = preserve_temp_failure(path);
            let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
            return Err(format!("El archivo exportado contiene {} pistas de video cuando se configuró solo audio{}", video_streams, diag_str));
        }
        if audio_streams == 0 {
            let diag = preserve_temp_failure(path);
            let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
            return Err(format!("El archivo exportado no contiene ninguna pista de audio{}", diag_str));
        }
    } else if video_streams == 0 {
        let diag = preserve_temp_failure(path);
        let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
        return Err(format!("El archivo exportado no contiene ninguna pista de video válida{}", diag_str));
    }

    // 5. Obtener duración del archivo verificado
    let probe_out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            path,
        ])
        .output();

    let duration = probe_out
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0);

    // 6. Validar duración si se especificó expectativa
    if let Some(exp_dur) = expected_duration {
        let tol = tolerance_sec.unwrap_or(0.5);
        let diff = (duration - exp_dur).abs();
        if diff > tol {
            let diag = preserve_temp_failure(path);
            let diag_str = diag.map(|d| format!(" (Archivo temporal conservado para diagnóstico en: {})", d)).unwrap_or_default();
            return Err(format!(
                "Discrepancia en la duración del archivo exportado: esperada {:.3}s, obtenida {:.3}s (desviación {:.3}s excede tolerancia {:.3}s){}",
                exp_dur, duration, diff, tol, diag_str
            ));
        }
    }

    Ok(VerifyReport {
        success: true,
        duration,
        file_size,
        video_streams,
        audio_streams,
    })
}

pub fn verify_output_file_detailed(
    path: &str,
    expected_duration: Option<f64>,
    tolerance_sec: Option<f64>,
    audio_only: bool,
) -> Result<VerifyReport, String> {
    verify_output_file_full(path, expected_duration, tolerance_sec, audio_only, None, None)
}

pub fn verify_output_file(path: &str) -> Result<VerifyReport, String> {
    verify_output_file_full(path, None, None, false, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_non_existent_file() {
        let res = verify_output_file("non_existent_file.mp4");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("no fue generado"));
    }

    #[test]
    fn test_vf10_verify_detects_wrong_streams_count() {
        let path = "RECURSOS/videoprueba.mp4";
        if !Path::new(path).exists() {
            return;
        }

        // videoprueba.mp4 tiene 1 video y 1 audio
        // Si exigimos 2 pistas de video, debe fallar con error descriptivo
        let res = verify_output_file_full(path, Some(15.0), Some(1.0), false, Some(2), Some(1));
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Discrepancia en pistas de video: esperadas 2, encontradas 1"));

        // Si exigimos solo audio en videoprueba.mp4, debe fallar porque tiene video
        let res_audio_only = verify_output_file_full(path, Some(15.0), Some(1.0), true, None, None);
        assert!(res_audio_only.is_err());
        assert!(res_audio_only.unwrap_err().contains("contiene 1 pistas de video"));
    }

    #[test]
    fn test_vf10_verify_detects_duration_discrepancy() {
        let path = "RECURSOS/videoprueba.mp4";
        if !Path::new(path).exists() {
            return;
        }

        // Duración esperada absurda de 99.0s vs real de 15.0s
        let res = verify_output_file_full(path, Some(99.0), Some(0.5), false, Some(1), Some(1));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Discrepancia en la duración"));
    }

    #[test]
    fn test_vf10_preserves_failed_diagnostic_file() {
        let temp_dir = std::env::temp_dir().join(format!("smartcut_test_verify_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let corrupt_file = temp_dir.join("corrupt_video.mp4");
        // Escribir archivo corrupto de más de 100 bytes para pasar el check de tamaño pero fallar ffprobe/ffmpeg
        let dummy_corrupt_data = vec![0xAB; 256];
        std::fs::write(&corrupt_file, dummy_corrupt_data).unwrap();

        let res = verify_output_file_full(&corrupt_file.to_string_lossy(), None, None, false, None, None);
        assert!(res.is_err(), "Corrupt file must fail verification");
        let err_msg = res.unwrap_err();
        assert!(
            err_msg.contains("Archivo temporal conservado para diagnóstico en:"),
            "Error must notify about preserved diagnostic file: {}",
            err_msg
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
