use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub static CURRENT_EXPORT_PID: Mutex<Option<u32>> = Mutex::new(None);
pub static IS_EXPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportProgress {
    pub percent: f64,
    pub stage: String,
    pub current_clip: usize,
    pub total_clips: usize,
    pub fps: f64,
    pub speed: String,
}

pub fn cancel_current_export() {
    IS_EXPORT_CANCELLED.store(true, Ordering::SeqCst);
    if let Ok(mut lock) = CURRENT_EXPORT_PID.lock() {
        if let Some(pid) = lock.take() {
            #[cfg(target_os = "windows")]
            let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).output();
            #[cfg(not(target_os = "windows"))]
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
        }
    }
}

pub fn is_cancelled() -> bool {
    IS_EXPORT_CANCELLED.load(Ordering::SeqCst)
}

pub fn reset_cancel_state() {
    IS_EXPORT_CANCELLED.store(false, Ordering::SeqCst);
}

pub fn run_ffmpeg_with_progress(
    args: &[String],
    expected_duration: f64,
    clip_index: usize,
    total_clips: usize,
    base_percent: f64,
    weight_percent: f64,
    stage_name: &str,
    app_handle: Option<&AppHandle>,
) -> Result<(), String> {
    if is_cancelled() {
        return Err("Exportación cancelada por el usuario".to_string());
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Error iniciando proceso FFmpeg: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "No se pudo capturar stdout de FFmpeg".to_string())?;
    let stderr = child.stderr.take();

    let pid = child.id();
    if let Ok(mut lock) = CURRENT_EXPORT_PID.lock() {
        *lock = Some(pid);
    }

    let stderr_lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let stderr_lines_clone = stderr_lines.clone();

    if let Some(err_pipe) = stderr {
        std::thread::spawn(move || {
            let reader = BufReader::new(err_pipe);
            for l in reader.lines().flatten() {
                if let Ok(mut buffer) = stderr_lines_clone.lock() {
                    if buffer.len() > 40 {
                        buffer.remove(0);
                    }
                    buffer.push(l);
                }
            }
        });
    }

    let reader = BufReader::new(stdout);
    let mut current_sec = 0.0;
    let mut fps = 0.0;
    let mut speed = "1.0x".to_string();

    for line_res in reader.lines() {
        if is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            if let Ok(mut lock) = CURRENT_EXPORT_PID.lock() {
                if *lock == Some(pid) {
                    *lock = None;
                }
            }
            return Err("Exportación cancelada por el usuario".to_string());
        }

        if let Ok(line) = line_res {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("out_time_us=") {
                if let Ok(us) = val.parse::<f64>() {
                    current_sec = us / 1_000_000.0;
                }
            } else if let Some(val) = line.strip_prefix("out_time_ms=") {
                if let Ok(ms) = val.parse::<f64>() {
                    current_sec = ms / 1_000_000.0;
                }
            } else if let Some(val) = line.strip_prefix("fps=") {
                if let Ok(f) = val.parse::<f64>() {
                    fps = f;
                }
            } else if let Some(val) = line.strip_prefix("speed=") {
                speed = val.to_string();
            } else if line == "progress=continue" || line == "progress=end" {
                let seg_prog = if expected_duration > 0.0 {
                    (current_sec / expected_duration).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let current_overall_percent =
                    (base_percent + seg_prog * weight_percent).clamp(0.0, 99.9);

                if let Some(handle) = app_handle {
                    let progress = ExportProgress {
                        percent: current_overall_percent,
                        stage: stage_name.to_string(),
                        current_clip: clip_index,
                        total_clips,
                        fps,
                        speed: speed.clone(),
                    };
                    let _ = handle.emit("export-progress", &progress);
                }
            }
        }
    }

    let status_res = child.wait();

    if let Ok(mut lock) = CURRENT_EXPORT_PID.lock() {
        if *lock == Some(pid) {
            *lock = None;
        }
    }

    match status_res {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else if is_cancelled() {
                Err("Exportación cancelada".to_string())
            } else {
                let err_details = stderr_lines
                    .lock()
                    .map(|b| b.join("\n"))
                    .unwrap_or_default();
                Err(format!("Error en FFmpeg:\n{}", err_details))
            }
        }
        Err(e) => Err(format!("Error esperando finalización de FFmpeg: {}", e)),
    }
}
