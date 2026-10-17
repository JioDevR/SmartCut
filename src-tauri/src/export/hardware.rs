use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HardwareEncoderInfo {
    pub id: String, // "nvenc", "qsv", "cpu", "copy"
    pub name: String,
    pub available: bool,
    pub description: String,
    pub is_recommended: bool,
}

#[tauri::command]
pub async fn get_available_encoders() -> Vec<HardwareEncoderInfo> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut encoders = Vec::new();

        // 1. Detectar NVIDIA NVENC
        let nvenc_test = Command::new("ffmpeg")
            .args(["-f", "lavfi", "-i", "testsrc=duration=1:size=256x256:rate=1", "-c:v", "h264_nvenc", "-f", "null", "-"])
            .output();
        let nvenc_available = nvenc_test.map(|o| o.status.success()).unwrap_or(false);

        let gpu_name = if nvenc_available {
            Command::new("nvidia-smi")
                .args(["--query-gpu=name", "--format=csv,noheader"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "NVIDIA GPU".to_string())
        } else {
            "NVIDIA GPU".to_string()
        };

        if nvenc_available {
            encoders.push(HardwareEncoderInfo {
                id: "nvenc".to_string(),
                name: format!("NVIDIA NVENC ({})", gpu_name),
                available: true,
                description: "GPU NVIDIA detectada. Aceleración por hardware de alta velocidad activada.".to_string(),
                is_recommended: true,
            });
        }

        // 2. Detectar Intel QSV
        let qsv_test = Command::new("ffmpeg")
            .args(["-f", "lavfi", "-i", "testsrc=duration=1:size=256x256:rate=1", "-c:v", "h264_qsv", "-f", "null", "-"])
            .output();
        let qsv_available = qsv_test.map(|o| o.status.success()).unwrap_or(false);

        if qsv_available {
            encoders.push(HardwareEncoderInfo {
                id: "qsv".to_string(),
                name: "Intel QuickSync Video (QSV)".to_string(),
                available: true,
                description: "GPU Intel detectada. Aceleración por hardware QuickSync activada.".to_string(),
                is_recommended: !nvenc_available,
            });
        }

        // 3. Procesamiento por CPU
        encoders.push(HardwareEncoderInfo {
            id: "cpu".to_string(),
            name: "CPU Multinúcleo (x264 / x265)".to_string(),
            available: true,
            description: "Renderizado estándar por procesador con compatibilidad universal.".to_string(),
            is_recommended: !nvenc_available && !qsv_available,
        });

        encoders
    })
    .await
    .unwrap_or_default()
}
