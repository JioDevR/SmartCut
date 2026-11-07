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

    let temp_dir = std::env::temp_dir().join(format!("smartcut_thumbs_{:016x}", video_hash));
    let _ = std::fs::create_dir_all(&temp_dir);

    let port = STREAM_SERVER_PORT.load(Ordering::SeqCst);
    let token = get_or_init_session_token();

    let duration = end_time - start_time;
    let mut times = Vec::with_capacity(count);
    for i in 0..count {
        let t = start_time + (i as f64 + 0.5) * (duration / count as f64);
        times.push(t);
    }

    // Paralelismo controlado con std::thread::scope (máx 4 hilos de extracción simultáneos)
    let num_workers = times.len().clamp(1, 4);
    let chunk_size = times.len().div_ceil(num_workers);
    let chunks: Vec<Vec<f64>> = times.chunks(chunk_size).map(|c| c.to_vec()).collect();

    let mut results: Vec<TimelineThumbnail> = std::thread::scope(|s| {
        let mut handles = Vec::new();

        for chunk in chunks {
            let temp_dir_ref = &temp_dir;
            let token_ref = &token;
            let path_ref = path;

            let handle = s.spawn(move || {
                let mut worker_results = Vec::new();
                for t in chunk {
                    // Clave redondeada a décimas de segundo para maximizar aciertos en caché
                    let time_key = (t * 10.0).round() as u64;
                    let thumb_filename = format!("smartcut_thumb_{:08}.jpg", time_key);
                    let mut dest_path = temp_dir_ref.join(&thumb_filename);
                    let legacy_path = temp_dir_ref.join(format!("thumb_{:08}.jpg", time_key));

                    let cached = if dest_path.exists() && dest_path.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                        true
                    } else if legacy_path.exists() && legacy_path.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                        dest_path = legacy_path;
                        true
                    } else {
                        false
                    };

                    if !cached {
                        // Búsqueda ultrarrápida al keyframe más cercano con -noaccurate_seek antes de -i
                        // Desactiva audio, subtítulos y datos con -an -sn -dn para decodificación inmediata
                        let _ = Command::new("ffmpeg")
                            .args([
                                "-noaccurate_seek",
                                "-ss",
                                &format!("{:.3}", t),
                                "-i",
                                path_ref,
                                "-frames:v",
                                "1",
                                "-an",
                                "-sn",
                                "-dn",
                                "-vf",
                                "scale=160:90:force_original_aspect_ratio=decrease",
                                "-q:v",
                                "6",
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
                        let path_str = dest_path.to_string_lossy().to_string();
                        crate::server::authorize_stream_path(&path_str);
                        let encoded_p = urlencoding_encode(&path_str);
                        worker_results.push(TimelineThumbnail {
                            time: t,
                            data_url: format!("http://127.0.0.1:{}/video?token={}&path={}", port, token_ref, encoded_p),
                        });
                    }
                }
                worker_results
            });
            handles.push(handle);
        }

        let mut combined = Vec::with_capacity(count);
        for h in handles {
            if let Ok(mut worker_res) = h.join() {
                combined.append(&mut worker_res);
            }
        }
        combined
    });

    results.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[tauri::command]
pub async fn get_timeline_thumbnails(path: String, duration: f64, count: Option<usize>) -> Vec<TimelineThumbnail> {
    if duration <= 0.0 {
        return Vec::new();
    }

    let num_thumbs = count.unwrap_or(24).clamp(8, 36);
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
