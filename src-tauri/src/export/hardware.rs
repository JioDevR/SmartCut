use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HardwareEncoderInfo {
    pub id: String, // "nvenc", "qsv", "amf", "vaapi", "videotoolbox", "cpu"
    pub name: String,
    pub short_tag: String,
    pub gpu_name: String,
    pub available: bool,
    pub description: String,
    pub is_recommended: bool,
}

fn detect_cpu_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in content.lines() {
                if line.starts_with("model name") {
                    if let Some(pos) = line.find(':') {
                        let name = line[pos + 1..].trim();
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = Command::new("wmic").args(["cpu", "get", "name"]).output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                let lines: Vec<&str> = s.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && *l != "Name").collect();
                if let Some(first) = lines.first() {
                    return first.to_string();
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    "Procesador Multinúcleo (CPU)".to_string()
}

fn detect_intel_gpu_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = Command::new("lspci").output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                for line in s.lines() {
                    let lower = line.to_lowercase();
                    if (lower.contains("vga") || lower.contains("3d") || lower.contains("display")) && lower.contains("intel") {
                        if let Some(start) = line.find('[') {
                            if let Some(end) = line[start..].find(']') {
                                let model = &line[start + 1..start + end];
                                return format!("Intel {}", model);
                            }
                        }
                        return "Intel UHD / Iris Graphics".to_string();
                    }
                }
            }
        }
    }

    "Intel QuickSync Graphics".to_string()
}

fn detect_amd_gpu_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = Command::new("lspci").output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                for line in s.lines() {
                    let lower = line.to_lowercase();
                    if (lower.contains("vga") || lower.contains("3d") || lower.contains("display"))
                        && (lower.contains("amd") || lower.contains("radeon") || lower.contains("advanced micro"))
                    {
                        if let Some(start) = line.find('[') {
                            if let Some(end) = line[start..].find(']') {
                                let model = &line[start + 1..start + end];
                                return format!("AMD {}", model);
                            }
                        }
                        return "AMD Radeon Graphics".to_string();
                    }
                }
            }
        }
    }

    "AMD Radeon Graphics".to_string()
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

        let nvidia_gpu_name = if nvenc_available {
            Command::new("nvidia-smi")
                .args(["--query-gpu=name", "--format=csv,noheader"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "NVIDIA GeForce GPU".to_string())
        } else {
            "NVIDIA GeForce GPU".to_string()
        };

        if nvenc_available {
            encoders.push(HardwareEncoderInfo {
                id: "nvenc".to_string(),
                name: "NVIDIA NVENC".to_string(),
                short_tag: "NVENC".to_string(),
                gpu_name: nvidia_gpu_name,
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
            let intel_gpu_name = detect_intel_gpu_name();
            encoders.push(HardwareEncoderInfo {
                id: "qsv".to_string(),
                name: "Intel QSV".to_string(),
                short_tag: "QSV".to_string(),
                gpu_name: intel_gpu_name,
                available: true,
                description: "GPU Intel detectada. Aceleración por hardware QuickSync activada.".to_string(),
                is_recommended: !nvenc_available,
            });
        }

        // 3. Detectar AMD AMF / VAAPI
        let amf_test = Command::new("ffmpeg")
            .args(["-f", "lavfi", "-i", "testsrc=duration=1:size=256x256:rate=1", "-c:v", "h264_amf", "-f", "null", "-"])
            .output();
        let amf_available = amf_test.map(|o| o.status.success()).unwrap_or(false);

        let vaapi_test = if !amf_available {
            Command::new("ffmpeg")
                .args(["-f", "lavfi", "-i", "testsrc=duration=1:size=256x256:rate=1", "-c:v", "h264_vaapi", "-f", "null", "-"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else {
            false
        };

        if amf_available || vaapi_test {
            let amd_gpu_name = detect_amd_gpu_name();
            let (enc_id, enc_name, short_tag) = if amf_available {
                ("amf", "AMD AMF", "AMF")
            } else {
                ("vaapi", "AMD VAAPI", "VAAPI")
            };

            encoders.push(HardwareEncoderInfo {
                id: enc_id.to_string(),
                name: enc_name.to_string(),
                short_tag: short_tag.to_string(),
                gpu_name: amd_gpu_name,
                available: true,
                description: "GPU AMD detectada. Aceleración por hardware activada.".to_string(),
                is_recommended: !nvenc_available && !qsv_available,
            });
        }

        // 4. Detectar Apple VideoToolbox (macOS)
        #[cfg(target_os = "macos")]
        {
            let vt_test = Command::new("ffmpeg")
                .args(["-f", "lavfi", "-i", "testsrc=duration=1:size=256x256:rate=1", "-c:v", "h264_videotoolbox", "-f", "null", "-"])
                .output();
            let vt_available = vt_test.map(|o| o.status.success()).unwrap_or(false);

            if vt_available {
                encoders.push(HardwareEncoderInfo {
                    id: "videotoolbox".to_string(),
                    name: "Apple VideoToolbox".to_string(),
                    short_tag: "Toolbox".to_string(),
                    gpu_name: "Apple Silicon GPU".to_string(),
                    available: true,
                    description: "Apple Silicon / GPU detectada. Aceleración por hardware nativa activada.".to_string(),
                    is_recommended: true,
                });
            }
        }

        let has_hw_accel = nvenc_available || qsv_available || amf_available || vaapi_test;

        // 5. Procesamiento por CPU
        let cpu_name = detect_cpu_name();
        encoders.push(HardwareEncoderInfo {
            id: "cpu".to_string(),
            name: "CPU (x264/x265)".to_string(),
            short_tag: "CPU".to_string(),
            gpu_name: cpu_name,
            available: true,
            description: "Renderizado estándar por procesador con compatibilidad universal.".to_string(),
            is_recommended: !has_hw_accel,
        });

        encoders
    })
    .await
    .unwrap_or_default()
}
