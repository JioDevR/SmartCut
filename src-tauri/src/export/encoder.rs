use super::probe::SourceInfo;

pub fn build_matching_encoder_args(source: &SourceInfo, is_mp4: bool) -> Vec<String> {
    let mut args = Vec::new();
    let video = match &source.video {
        Some(v) => v,
        None => {
            args.extend(["-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), "15".to_string(), "-preset".to_string(), "fast".to_string()]);
            return args;
        }
    };

    let codec = video.codec_name.to_lowercase();
    let is_hevc = codec == "hevc" || codec == "h265";

    if is_hevc {
        args.extend(["-c:v".to_string(), "libx265".to_string(), "-crf".to_string(), "17".to_string(), "-preset".to_string(), "fast".to_string()]);

        // Manejo de perfil y 10-bit para HEVC
        let is_10bit = video.pix_fmt.contains("10") || video.bits_per_raw_sample.unwrap_or(8) >= 10;
        if is_10bit {
            args.extend(["-pix_fmt".to_string(), "yuv420p10le".to_string(), "-profile:v".to_string(), "main10".to_string()]);
        } else {
            args.extend(["-pix_fmt".to_string(), video.pix_fmt.clone()]);
            if let Some(ref prof) = video.profile {
                args.extend(["-profile:v".to_string(), prof.to_lowercase()]);
            }
        }

        let mut x265_params = vec!["no-open-gop=1".to_string(), "repeat-headers=1".to_string()];
        if let Some(lvl) = video.level {
            if lvl >= 30 && lvl <= 186 && lvl % 3 == 0 {
                let lvl_float = (lvl as f64) / 30.0;
                x265_params.push(format!("level-idc={:.1}", lvl_float));
            } else if lvl >= 10 && lvl <= 62 {
                x265_params.push(format!("level-idc={}", lvl));
            }
        }
        args.extend(["-x265-params".to_string(), x265_params.join(":")]);

        if is_mp4 {
            let tag = video.codec_tag_string.as_deref().unwrap_or("hvc1");
            args.extend(["-tag:v".to_string(), tag.to_string()]);
        }
    } else {
        // H.264 / AVC
        args.extend(["-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), "15".to_string(), "-preset".to_string(), "fast".to_string()]);

        args.extend(["-pix_fmt".to_string(), video.pix_fmt.clone()]);

        if let Some(ref prof) = video.profile {
            let prof_lower = prof.to_lowercase();
            if prof_lower.contains("high") {
                args.extend(["-profile:v".to_string(), "high".to_string()]);
            } else if prof_lower.contains("main") {
                args.extend(["-profile:v".to_string(), "main".to_string()]);
            } else if prof_lower.contains("baseline") {
                args.extend(["-profile:v".to_string(), "baseline".to_string()]);
            }
        }

        if let Some(lvl) = video.level {
            // ffmpeg espera niveles como "4.1", "3.2" o "41"
            let lvl_str = if lvl >= 10 && lvl <= 62 {
                format!("{:.1}", lvl as f64 / 10.0)
            } else {
                lvl.to_string()
            };
            args.extend(["-level".to_string(), lvl_str]);
        }

        args.extend([
            "-flags".to_string(),
            "+cgop".to_string(),
            "-x264-params".to_string(),
            "open-gop=0:no-scenecut=1".to_string(),
            "-bsf:v".to_string(),
            "dump_extra".to_string(),
        ]);
    }

    // Color metadata
    if let Some(ref cr) = video.color_range {
        args.extend(["-color_range".to_string(), cr.clone()]);
    }
    if let Some(ref cs) = video.color_space {
        args.extend(["-colorspace".to_string(), cs.clone()]);
    }
    if let Some(ref cp) = video.color_primaries {
        args.extend(["-color_primaries".to_string(), cp.clone()]);
    }
    if let Some(ref ct) = video.color_trc {
        args.extend(["-color_trc".to_string(), ct.clone()]);
    }
    if let Some(ref sar) = video.sar {
        args.extend(["-sar".to_string(), sar.clone()]);
    }

    // Manejo de FPS
    if video.is_vfr {
        args.extend(["-fps_mode".to_string(), "passthrough".to_string()]);
    } else if video.fps > 0.0 {
        args.extend(["-r".to_string(), video.r_frame_rate.clone()]);
    }

    args
}
