use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PacketInfo {
    pub pts: i64,
    pub pts_time: f64,
    pub dts: i64,
    pub dts_time: f64,
    pub duration: i64,
    pub duration_time: f64,
    pub is_key: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct KeyframeInfo {
    pub pts: i64,
    pub pts_time: f64,
    pub is_open_gop: bool,
    pub min_leading_pts: i64,
    pub min_leading_time: f64,
}

pub fn read_packets_in_interval(
    path: &str,
    stream_idx: usize,
    start_sec: f64,
    end_sec: f64,
) -> Result<Vec<PacketInfo>, String> {
    let start_safe = (start_sec - 0.2).max(0.0);
    let end_safe = end_sec + 0.2;
    let interval_arg = format!("{:.6}%{:.6}", start_safe, end_safe);

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            &format!("v:{}", stream_idx),
            "-show_entries",
            "packet=pts,pts_time,dts,dts_time,duration,duration_time,flags",
            "-of",
            "csv=p=0",
            "-read_intervals",
            &interval_arg,
            path,
        ])
        .output()
        .map_err(|e| format!("Error ejecutando ffprobe para paquetes: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe lectura de paquetes falló: {}", err));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut packets = Vec::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }

        // pts,pts_time,dts,dts_time,duration,duration_time,flags
        let pts = parts.first().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        let pts_time = parts.get(1).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        let dts = parts.get(2).and_then(|v| v.parse::<i64>().ok()).unwrap_or(pts);
        let dts_time = parts.get(3).and_then(|v| v.parse::<f64>().ok()).unwrap_or(pts_time);
        let duration = parts.get(4).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        let duration_time = parts.get(5).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        let flags = parts.get(6).copied().unwrap_or("");
        let is_key = flags.starts_with('K');

        packets.push(PacketInfo {
            pts,
            pts_time,
            dts,
            dts_time,
            duration,
            duration_time,
            is_key,
        });
    }

    Ok(packets)
}

pub fn get_keyframes_with_gop_info(
    path: &str,
    stream_idx: usize,
    start_sec: f64,
    end_sec: f64,
) -> Result<Vec<KeyframeInfo>, String> {
    let packets = read_packets_in_interval(path, stream_idx, start_sec, end_sec)?;
    if packets.is_empty() {
        return Ok(Vec::new());
    }

    let mut keyframes = Vec::new();

    for (idx, pkt) in packets.iter().enumerate() {
        if pkt.is_key {
            let mut is_open = false;
            let mut min_pts = pkt.pts;
            let mut min_time = pkt.pts_time;

            // Revisar paquetes que siguen en orden de decodificación hasta el próximo keyframe
            for next_pkt in packets.iter().skip(idx + 1) {
                if next_pkt.is_key {
                    break;
                }
                if next_pkt.pts < pkt.pts {
                    is_open = true;
                    if next_pkt.pts < min_pts {
                        min_pts = next_pkt.pts;
                        min_time = next_pkt.pts_time;
                    }
                }
            }

            keyframes.push(KeyframeInfo {
                pts: pkt.pts,
                pts_time: pkt.pts_time,
                is_open_gop: is_open,
                min_leading_pts: min_pts,
                min_leading_time: min_time,
            });
        }
    }

    keyframes.sort_by(|a, b| a.pts_time.partial_cmp(&b.pts_time).unwrap_or(std::cmp::Ordering::Equal));
    keyframes.dedup_by(|a, b| (a.pts_time - b.pts_time).abs() < 0.001);

    Ok(keyframes)
}
