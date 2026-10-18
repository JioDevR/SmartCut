use super::probe::SourceInfo;

pub fn build_encoder_args(
    source: &SourceInfo,
    is_mp4: bool,
    hardware_accel: Option<&str>,
    crf: Option<u32>,
    preset: Option<&str>,
    target_codec: Option<&str>,
) -> Vec<String> {
    let mut args = Vec::new();
    let video = source.video.as_ref();

    let codec = target_codec
        .map(|c| c.to_lowercase())
        .or_else(|| video.map(|v| v.codec_name.to_lowercase()))
        .unwrap_or_else(|| "h264".to_string());

    let is_hevc = codec == "hevc" || codec == "h265";
    let is_vp9 = codec == "vp9" || codec == "libvpx-vp9";
    let is_10bit = video
        .map(|v| v.pix_fmt.contains("10") || v.bits_per_raw_sample.unwrap_or(8) >= 10)
        .unwrap_or(false);

    let accel = hardware_accel.unwrap_or("cpu").to_lowercase();

    match accel.as_str() {
        "nvenc" => {
            if is_hevc {
                args.extend(["-c:v".to_string(), "hevc_nvenc".to_string()]);
            } else {
                args.extend(["-c:v".to_string(), "h264_nvenc".to_string()]);
            }

            let q = crf.unwrap_or(20);
            args.extend(["-cq".to_string(), q.to_string()]);

            let p = preset.unwrap_or("fast");
            args.extend(["-preset".to_string(), p.to_string()]);

            if is_10bit {
                args.extend(["-pix_fmt".to_string(), "p010le".to_string()]);
            } else if let Some(v) = video {
                args.extend(["-pix_fmt".to_string(), v.pix_fmt.clone()]);
            }

            if is_hevc && is_mp4 {
                let tag = video.and_then(|v| v.codec_tag_string.clone()).unwrap_or_else(|| "hvc1".to_string());
                args.extend(["-tag:v".to_string(), tag]);
            }
        }
        "qsv" => {
            if is_hevc {
                args.extend(["-c:v".to_string(), "hevc_qsv".to_string()]);
            } else {
                args.extend(["-c:v".to_string(), "h264_qsv".to_string()]);
            }

            let q = crf.unwrap_or(20);
            args.extend(["-global_quality".to_string(), q.to_string()]);

            let p = preset.unwrap_or("fast");
            args.extend(["-preset".to_string(), p.to_string()]);

            if let Some(v) = video {
                args.extend(["-pix_fmt".to_string(), v.pix_fmt.clone()]);
            }

            if is_hevc && is_mp4 {
                let tag = video.and_then(|v| v.codec_tag_string.clone()).unwrap_or_else(|| "hvc1".to_string());
                args.extend(["-tag:v".to_string(), tag]);
            }
        }
        _ => {
            // CPU fallback (libx264, libx265, libvpx-vp9)
            if is_hevc {
                let q = crf.unwrap_or(17);
                let p = preset.unwrap_or("fast");
                args.extend([
                    "-c:v".to_string(),
                    "libx265".to_string(),
                    "-crf".to_string(),
                    q.to_string(),
                    "-preset".to_string(),
                    p.to_string(),
                ]);

                if is_10bit {
                    args.extend(["-pix_fmt".to_string(), "yuv420p10le".to_string(), "-profile:v".to_string(), "main10".to_string()]);
                } else if let Some(v) = video {
                    args.extend(["-pix_fmt".to_string(), v.pix_fmt.clone()]);
                    if let Some(ref prof) = v.profile {
                        args.extend(["-profile:v".to_string(), prof.to_lowercase()]);
                    }
                }

                let mut x265_params = vec!["no-open-gop=1".to_string(), "repeat-headers=1".to_string()];
                if let Some(v) = video {
                    if let Some(lvl) = v.level {
                        if lvl >= 30 && lvl <= 186 && lvl % 3 == 0 {
                            let lvl_float = (lvl as f64) / 30.0;
                            x265_params.push(format!("level-idc={:.1}", lvl_float));
                        } else if lvl >= 10 && lvl <= 62 {
                            x265_params.push(format!("level-idc={}", lvl));
                        }
                    }
                }
                args.extend(["-x265-params".to_string(), x265_params.join(":")]);

                if is_mp4 {
                    let tag = video.and_then(|v| v.codec_tag_string.clone()).unwrap_or_else(|| "hvc1".to_string());
                    args.extend(["-tag:v".to_string(), tag]);
                }
            } else if is_vp9 {
                let q = crf.unwrap_or(24);
                args.extend([
                    "-c:v".to_string(),
                    "libvpx-vp9".to_string(),
                    "-crf".to_string(),
                    q.to_string(),
                    "-b:v".to_string(),
                    "0".to_string(),
                ]);
                if let Some(v) = video {
                    args.extend(["-pix_fmt".to_string(), v.pix_fmt.clone()]);
                }
            } else {
                // H.264
                let q = crf.unwrap_or(18);
                let p = preset.unwrap_or("fast");
                args.extend([
                    "-c:v".to_string(),
                    "libx264".to_string(),
                    "-crf".to_string(),
                    q.to_string(),
                    "-preset".to_string(),
                    p.to_string(),
                ]);

                if let Some(v) = video {
                    args.extend(["-pix_fmt".to_string(), v.pix_fmt.clone()]);

                    if let Some(ref prof) = v.profile {
                        let prof_lower = prof.to_lowercase();
                        if prof_lower.contains("high") {
                            args.extend(["-profile:v".to_string(), "high".to_string()]);
                        } else if prof_lower.contains("main") {
                            args.extend(["-profile:v".to_string(), "main".to_string()]);
                        } else if prof_lower.contains("baseline") {
                            args.extend(["-profile:v".to_string(), "baseline".to_string()]);
                        }
                    }

                    if let Some(lvl) = v.level {
                        let lvl_str = if lvl >= 10 && lvl <= 62 {
                            format!("{:.1}", lvl as f64 / 10.0)
                        } else {
                            lvl.to_string()
                        };
                        args.extend(["-level".to_string(), lvl_str]);
                    }
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
        }
    }

    // Color metadata and fps from source video if available
    if let Some(v) = video {
        if let Some(ref cr) = v.color_range {
            args.extend(["-color_range".to_string(), cr.clone()]);
        }
        if let Some(ref cs) = v.color_space {
            args.extend(["-colorspace".to_string(), cs.clone()]);
        }
        if let Some(ref cp) = v.color_primaries {
            args.extend(["-color_primaries".to_string(), cp.clone()]);
        }
        if let Some(ref ct) = v.color_trc {
            args.extend(["-color_trc".to_string(), ct.clone()]);
        }
        if let Some(ref sar) = v.sar {
            args.extend(["-sar".to_string(), sar.clone()]);
        }

        if v.is_vfr {
            args.extend(["-fps_mode".to_string(), "passthrough".to_string()]);
        } else if v.fps > 0.0 {
            args.extend(["-r".to_string(), v.r_frame_rate.clone()]);
        }
    }

    args
}

pub fn build_matching_encoder_args(source: &SourceInfo, is_mp4: bool) -> Vec<String> {
    build_encoder_args(source, is_mp4, None, None, None, None)
}
