use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::Ordering;

use crate::server::{get_or_init_session_token, urlencoding_encode, STREAM_SERVER_PORT};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimelineThumbnail {
    pub time: f64,
    pub data_url: String,
}

pub fn extract_thumbnails_for_interval(
    path: &str,
    start_time: f64,
    end_time: f64,
    count: usize,
) -> Vec<TimelineThumbnail> {
    let p = Path::new(path);
    if !p.exists() || end_time <= start_time || count == 0 {
        return Vec::new();
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
    let video_hash = hasher.finish();

    let temp_dir = std::env::temp_dir();
    let port = STREAM_SERVER_PORT.load(Ordering::SeqCst);
    let token = get_or_init_session_token();

    let duration = end_time - start_time;
    let mut results = Vec::with_capacity(count);

    for i in 0..count {
        let t = start_time + (i as f64 + 0.5) * (duration / count as f64);
        let time_key = (t * 10.0).round() as u64;
        let thumb_filename = format!("smartcut_thumb_{:016x}_{:08}.jpg", video_hash, time_key);
        let dest_path = temp_dir.join(&thumb_filename);

        let cached = dest_path.exists() && dest_path.metadata().map(|m| m.len() > 100).unwrap_or(false);
        if !cached {
            // Extracción ultrarrápida por Keyframe (-ss antes de -i sin decodificar todo el video linealmente)
            let _ = Command::new("ffmpeg")
                .args([
                    "-ss",
                    &format!("{:.3}", t),
                    "-i",
                    path,
                    "-frames:v",
                    "1",
                    "-vf",
                    "scale=160:90:force_original_aspect_ratio=decrease",
                    "-q:v",
                    "5",
                    "-threads",
                    "1",
                    "-v",
                    "error",
                    "-y",
                    dest_path.to_str().unwrap_or(""),
                ])
                .output();
        }

        if dest_path.exists() && dest_path.metadata().map(|m| m.len() > 100).unwrap_or(false) {
            let encoded_p = urlencoding_encode(&dest_path.to_string_lossy());
            results.push(TimelineThumbnail {
                time: t,
                data_url: format!("http://127.0.0.1:{}/video?token={}&path={}", port, token, encoded_p),
            });
        }
    }

    results
}

#[tauri::command]
pub async fn get_timeline_thumbnails(path: String, duration: f64, count: Option<usize>) -> Vec<TimelineThumbnail> {
    if duration <= 0.0 {
        return Vec::new();
    }

    let num_thumbs = count.unwrap_or(12).clamp(4, 24);
    tauri::async_runtime::spawn_blocking(move || {
        extract_thumbnails_for_interval(&path, 0.0, duration, num_thumbs)
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn get_thumbnails_for_range(
    path: String,
    start_time: f64,
    end_time: f64,
    count: usize,
) -> Vec<TimelineThumbnail> {
    if start_time < 0.0 || end_time <= start_time {
        return Vec::new();
    }

    let num_thumbs = count.clamp(1, 24);
    tauri::async_runtime::spawn_blocking(move || {
        extract_thumbnails_for_interval(&path, start_time, end_time, num_thumbs)
    })
    .await
    .unwrap_or_default()
}
