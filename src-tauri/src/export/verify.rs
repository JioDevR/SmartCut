use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyReport {
    pub success: bool,
    pub duration: f64,
    pub file_size: u64,
}

pub fn verify_output_file(path: &str) -> Result<VerifyReport, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("El archivo de salida no fue generado: {}", path));
    }

    let file_size = p.metadata().map(|m| m.len()).unwrap_or(0);
    if file_size < 1024 {
        return Err(format!("El archivo de salida está vacío o corrupto (tamaño {} bytes)", file_size));
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
            return Err(format!("Verificación de integridad falló en el video generado: {}", err));
        }
    }

    // 2. Obtener duración del archivo verificado
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

    Ok(VerifyReport {
        success: true,
        duration,
        file_size,
    })
}
