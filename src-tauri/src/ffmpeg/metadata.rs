use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VideoMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub format: String,
    pub video_codec: String,
    pub audio_codec: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SampleVideoInfo {
    pub name: String,
    pub path: String,
    pub format: String,
    pub size_formatted: String,
}

pub fn parse_fps(fps_str: &str) -> f64 {
    if let Some((num, den)) = fps_str.split_once('/') {
        let n: f64 = num.parse().unwrap_or(30.0);
        let d: f64 = den.parse().unwrap_or(1.0);
        if d > 0.0 {
            n / d
        } else {
            30.0
        }
    } else {
        fps_str.parse().unwrap_or(30.0)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[tauri::command]
pub async fn get_video_metadata(path: String) -> Result<VideoMetadata, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = Path::new(&path);
        if !p.exists() {
            return Err(format!("El archivo no existe: {}", path));
        }

        let file_size = p.metadata().map(|m| m.len()).unwrap_or(0);
        let file_name = p
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "video".to_string());

        let mut duration = 0.0;
        let mut width = 0;
        let mut height = 0;
        let mut fps = 30.0;
        let mut format = p
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "desconocido".to_string());
        let mut video_codec = "desconocido".to_string();
        let mut audio_codec = "desconocido".to_string();

        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name,codec_type,width,height,r_frame_rate,duration",
                "-show_entries",
                "format=duration,format_name,size",
                "-of",
                "json",
                &path,
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                if let Ok(json_str) = String::from_utf8(out.stdout) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if let Some(fmt) = val.get("format") {
                            if let Some(d) = fmt.get("duration").and_then(|v| v.as_str()) {
                                duration = d.parse().unwrap_or(0.0);
                            }
                            if let Some(fn_val) = fmt.get("format_name").and_then(|v| v.as_str()) {
                                format = fn_val.to_string();
                            }
                        }

                        if let Some(streams) = val.get("streams").and_then(|s| s.as_array()) {
                            for st in streams {
                                let codec_type = st
                                    .get("codec_type")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("");
                                if codec_type == "video" && video_codec == "desconocido" {
                                    if let Some(c) = st.get("codec_name").and_then(|v| v.as_str()) {
                                        video_codec = c.to_string();
                                    }
                                    if let Some(w) = st.get("width").and_then(|v| v.as_u64()) {
                                        width = w as u32;
                                    }
                                    if let Some(h) = st.get("height").and_then(|v| v.as_u64()) {
                                        height = h as u32;
                                    }
                                    if let Some(rf) = st.get("r_frame_rate").and_then(|v| v.as_str()) {
                                        fps = parse_fps(rf);
                                    }
                                    if duration == 0.0 {
                                        if let Some(d) = st.get("duration").and_then(|v| v.as_str()) {
                                            duration = d.parse().unwrap_or(0.0);
                                        }
                                    }
                                } else if codec_type == "audio" && audio_codec == "desconocido" {
                                    if let Some(c) = st.get("codec_name").and_then(|v| v.as_str()) {
                                        audio_codec = c.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(VideoMetadata {
            file_path: path,
            file_name,
            file_size,
            duration,
            width,
            height,
            fps,
            format,
            video_codec,
            audio_codec,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_sample_videos() -> Vec<SampleVideoInfo> {
    let mut list = Vec::new();

    let candidates = [
        ("videoprueba.mp4", "Prueba MP4 (H.264 / AAC)", "MP4"),
        ("videoprueba.mkv", "Prueba MKV (HEVC / AAC - 328MB)", "MKV"),
    ];

    let current_dir = std::env::current_dir().unwrap_or_default();
    let parent_dir = current_dir.parent().unwrap_or(&current_dir);

    for (file_name, label, fmt) in candidates {
        let p1 = current_dir.join("RECURSOS").join(file_name);
        let p2 = current_dir.join(file_name);
        let p3 = parent_dir.join("RECURSOS").join(file_name);

        let final_path = if p1.exists() {
            Some(p1)
        } else if p2.exists() {
            Some(p2)
        } else if p3.exists() {
            Some(p3)
        } else {
            None
        };

        if let Some(p) = final_path {
            let size = p.metadata().map(|m| m.len()).unwrap_or(0);
            let size_str = format_bytes(size);
            let canonical = p.canonicalize().unwrap_or(p);
            list.push(SampleVideoInfo {
                name: label.to_string(),
                path: canonical.to_string_lossy().to_string(),
                format: fmt.to_string(),
                size_formatted: size_str,
            });
        }
    }

    list
}
