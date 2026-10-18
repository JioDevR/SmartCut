use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VideoStreamInfo {
    pub stream_index: usize,
    pub video_index: usize,
    pub codec_name: String,
    pub profile: Option<String>,
    pub level: Option<i64>,
    pub pix_fmt: String,
    pub bits_per_raw_sample: Option<u32>,
    pub width: u32,
    pub height: u32,
    pub sar: Option<String>,
    pub color_range: Option<String>,
    pub color_space: Option<String>,
    pub color_primaries: Option<String>,
    pub color_trc: Option<String>,
    pub r_frame_rate: String,
    pub avg_frame_rate: String,
    pub fps: f64,
    pub time_base: String,
    pub time_base_num: i64,
    pub time_base_den: i64,
    pub start_time: f64,
    pub start_pts: i64,
    pub codec_tag_string: Option<String>,
    pub rotation: i64,
    pub is_vfr: bool,
    pub has_b_frames: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AudioStreamInfo {
    pub stream_index: usize,
    pub audio_index: usize,
    pub codec_name: String,
    pub profile: Option<String>,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: Option<String>,
    pub bit_rate: Option<u64>,
    pub time_base: String,
    pub start_time: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SourceInfo {
    pub path: String,
    pub container_ext: String,
    pub format_name: String,
    pub duration: f64,
    pub start_time: f64,
    pub video: Option<VideoStreamInfo>,
    pub audio: Vec<AudioStreamInfo>,
}

pub fn parse_rational(s: &str) -> Option<(i64, i64)> {
    if let Some((n, d)) = s.split_once('/') {
        let num = n.trim().parse::<i64>().ok()?;
        let den = d.trim().parse::<i64>().ok()?;
        if den != 0 {
            Some((num, den))
        } else {
            None
        }
    } else {
        let num = s.trim().parse::<i64>().ok()?;
        Some((num, 1))
    }
}

pub fn rational_to_f64(s: &str) -> f64 {
    if let Some((n, d)) = parse_rational(s) {
        if d != 0 {
            return n as f64 / d as f64;
        }
    }
    s.parse::<f64>().unwrap_or(0.0)
}

pub fn probe_source_info(path: &str) -> Result<SourceInfo, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("El archivo no existe: {}", path));
    }

    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "mp4".to_string());

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
            path,
        ])
        .output()
        .map_err(|e| format!("Error ejecutando ffprobe: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe falló: {}", err));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Error parseando json de ffprobe: {}", e))?;

    let mut info = SourceInfo {
        path: path.to_string(),
        container_ext: ext,
        ..Default::default()
    };

    if let Some(fmt) = val.get("format") {
        if let Some(fn_val) = fmt.get("format_name").and_then(|v| v.as_str()) {
            info.format_name = fn_val.to_string();
        }
        if let Some(d) = fmt.get("duration").and_then(|v| v.as_str()) {
            info.duration = d.parse::<f64>().unwrap_or(0.0);
        }
        if let Some(st) = fmt.get("start_time").and_then(|v| v.as_str()) {
            info.start_time = st.parse::<f64>().unwrap_or(0.0);
        }
    }

    if let Some(streams) = val.get("streams").and_then(|s| s.as_array()) {
        let mut video_count = 0;
        let mut audio_count = 0;
        for (idx, st) in streams.iter().enumerate() {
            let codec_type = st.get("codec_type").and_then(|v| v.as_str()).unwrap_or("");
            let stream_index = st.get("index").and_then(|v| v.as_u64()).map(|u| u as usize).unwrap_or(idx);

            if codec_type == "video" {
                if info.video.is_none() {
                    let codec_name = st.get("codec_name").and_then(|v| v.as_str()).unwrap_or("h264").to_string();
                    let profile = st.get("profile").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let level = st.get("level").and_then(|v| v.as_i64());
                    let pix_fmt = st.get("pix_fmt").and_then(|v| v.as_str()).unwrap_or("yuv420p").to_string();
                    let width = st.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let height = st.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let sar = st.get("sample_aspect_ratio").and_then(|v| v.as_str()).filter(|s| *s != "0:1" && *s != "unknown").map(|s| s.to_string());

                    let color_range = st.get("color_range").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let color_space = st.get("color_space").and_then(|v| v.as_str()).filter(|s| *s != "unknown" && *s != "unspecified").map(|s| s.to_string());
                    let color_primaries = st.get("color_primaries").and_then(|v| v.as_str()).filter(|s| *s != "unknown" && *s != "unspecified").map(|s| s.to_string());
                    let color_trc = st.get("color_transfer").and_then(|v| v.as_str()).filter(|s| *s != "unknown" && *s != "unspecified").map(|s| s.to_string());

                    let r_frame_rate = st.get("r_frame_rate").and_then(|v| v.as_str()).unwrap_or("30/1").to_string();
                    let avg_frame_rate = st.get("avg_frame_rate").and_then(|v| v.as_str()).unwrap_or("30/1").to_string();
                    let r_fps = rational_to_f64(&r_frame_rate);
                    let avg_fps = rational_to_f64(&avg_frame_rate);
                    let fps = if r_fps > 0.0 { r_fps } else if avg_fps > 0.0 { avg_fps } else { 30.0 };

                    let is_vfr = (r_fps > 0.0 && avg_fps > 0.0) && ((r_fps - avg_fps).abs() / avg_fps > 0.02);

                    let time_base = st.get("time_base").and_then(|v| v.as_str()).unwrap_or("1/90000").to_string();
                    let (tb_num, tb_den) = parse_rational(&time_base).unwrap_or((1, 90000));

                    let start_time = st.get("start_time").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(info.start_time);
                    let start_pts = st.get("start_pts").and_then(|v| v.as_i64()).unwrap_or(0);

                    let codec_tag_string = st.get("codec_tag_string").and_then(|v| v.as_str()).filter(|s| *s != "[0][0][0][0]").map(|s| s.to_string());

                    let bits_per_raw_sample = st.get("bits_per_raw_sample").and_then(|v| v.as_str()).and_then(|s| s.parse::<u32>().ok());
                    let has_b_frames = st.get("has_b_frames").and_then(|v| v.as_i64()).unwrap_or(0) > 0;

                    // Rotación: revisar tags y side_data_list
                    let mut rotation: i64 = 0;
                    if let Some(tags) = st.get("tags") {
                        if let Some(rot_str) = tags.get("rotate").and_then(|v| v.as_str()) {
                            rotation = rot_str.parse::<i64>().unwrap_or(0);
                        }
                    }
                    if rotation == 0 {
                        if let Some(side_data) = st.get("side_data_list").and_then(|v| v.as_array()) {
                            for sd in side_data {
                                if let Some(rot) = sd.get("rotation").and_then(|r| r.as_i64()) {
                                    rotation = rot;
                                    break;
                                }
                            }
                        }
                    }

                    info.video = Some(VideoStreamInfo {
                        stream_index,
                        video_index: video_count,
                        codec_name,
                        profile,
                        level,
                        pix_fmt,
                        bits_per_raw_sample,
                        width,
                        height,
                        sar,
                        color_range,
                        color_space,
                        color_primaries,
                        color_trc,
                        r_frame_rate,
                        avg_frame_rate,
                        fps,
                        time_base,
                        time_base_num: tb_num,
                        time_base_den: tb_den,
                        start_time,
                        start_pts,
                        codec_tag_string,
                        rotation,
                        is_vfr,
                        has_b_frames,
                    });
                }
                video_count += 1;
            } else if codec_type == "audio" {
                let codec_name = st.get("codec_name").and_then(|v| v.as_str()).unwrap_or("aac").to_string();
                let profile = st.get("profile").and_then(|v| v.as_str()).map(|s| s.to_string());
                let sample_rate = st.get("sample_rate").and_then(|v| v.as_str()).and_then(|s| s.parse::<u32>().ok()).unwrap_or(48000);
                let channels = st.get("channels").and_then(|v| v.as_u64()).unwrap_or(2) as u32;
                let channel_layout = st.get("channel_layout").and_then(|v| v.as_str()).filter(|s| *s != "unknown").map(|s| s.to_string());
                let bit_rate = st.get("bit_rate").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok());
                let time_base = st.get("time_base").and_then(|v| v.as_str()).unwrap_or("1/48000").to_string();
                let start_time = st.get("start_time").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);

                info.audio.push(AudioStreamInfo {
                    stream_index,
                    audio_index: audio_count,
                    codec_name,
                    profile,
                    sample_rate,
                    channels,
                    channel_layout,
                    bit_rate,
                    time_base,
                    start_time,
                });
                audio_count += 1;
            }
        }
    }

    Ok(info)
}
