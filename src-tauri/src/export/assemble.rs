use std::path::Path;
use tauri::AppHandle;

use super::encoder::build_matching_encoder_args;
use super::planner::{FullExportPlan, PieceKind};
use super::probe::SourceInfo;
use super::runner::{is_cancelled, run_ffmpeg_with_progress};
use super::temp::TempDir;
use super::verify::verify_output_file;

pub fn execute_smartcut_export(
    source: &SourceInfo,
    plan: &FullExportPlan,
    output_path: &str,
    temp_dir: &TempDir,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    if is_cancelled() {
        return Err("Exportación cancelada".to_string());
    }

    let out_p = Path::new(output_path);
    if let Some(parent) = out_p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let ext = out_p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| source.container_ext.clone());

    let is_mp4 = ext == "mp4" || ext == "mov" || ext == "m4v";
    let video_idx = source.video.as_ref().map(|v| v.stream_index).unwrap_or(0);

    // 1. Extraer / renderizar piezas de video
    let mut total_pieces = 0;
    for seg in &plan.segments {
        total_pieces += seg.video_pieces.len();
    }
    if total_pieces == 0 {
        return Err("El plan de exportación no contiene video para procesar".to_string());
    }

    let mut video_piece_files = Vec::new();
    let mut global_piece_idx = 0;

    for seg in &plan.segments {
        for piece in &seg.video_pieces {
            if is_cancelled() {
                return Err("Exportación cancelada".to_string());
            }

            let piece_filename = format!("video_piece_{:04}.ts", global_piece_idx);
            let piece_path = temp_dir.file_path(&piece_filename);
            let piece_path_str = piece_path.to_string_lossy().to_string();
            video_piece_files.push(piece_filename);

            let base_pct = (global_piece_idx as f64 / total_pieces as f64) * 75.0;
            let weight_pct = 75.0 / total_pieces as f64;
            let stage = format!("Smart Cut: Clip {} ({})", seg.segment_index + 1, piece.stage_desc);

            let mut cmd = vec!["-y".to_string()];

            match piece.kind {
                PieceKind::Encode => {
                    cmd.extend([
                        "-noautorotate".to_string(),
                        "-ss".to_string(),
                        format!("{:.6}", piece.start_sec),
                        "-i".to_string(),
                        source.path.clone(),
                        "-t".to_string(),
                        format!("{:.6}", piece.duration_sec),
                        "-map".to_string(),
                        format!("0:v:{}", video_idx),
                    ]);
                    cmd.extend(build_matching_encoder_args(source, is_mp4));
                    cmd.extend([
                        "-an".to_string(),
                        "-f".to_string(),
                        "mpegts".to_string(),
                        "-progress".to_string(),
                        "pipe:1".to_string(),
                        piece_path_str,
                    ]);
                }
                PieceKind::Copy => {
                    let fps = source.video.as_ref().map(|v| v.fps).unwrap_or(30.0);
                    let safe_fps = if fps > 0.0 { fps } else { 30.0 };
                    let frame_dur = 1.0 / safe_fps;
                    let has_b_frames = source.video.as_ref().map(|v| v.has_b_frames).unwrap_or(false);
                    let trim_margin = if has_b_frames { frame_dur * 2.5 } else { frame_dur * 0.75 };
                    let safe_dur = (piece.duration_sec - trim_margin).max(frame_dur);

                    let seek_offset = if has_b_frames { frame_dur * 4.5 } else { frame_dur * 0.5 };

                    cmd.extend([
                        "-ss".to_string(),
                        format!("{:.6}", piece.start_sec + seek_offset),
                        "-i".to_string(),
                        source.path.clone(),
                        "-t".to_string(),
                        format!("{:.6}", safe_dur),
                        "-map".to_string(),
                        format!("0:v:{}", video_idx),
                        "-c:v".to_string(),
                        "copy".to_string(),
                        "-avoid_negative_ts".to_string(),
                        "make_zero".to_string(),
                    ]);
                    if piece.drop_leading {
                        cmd.extend(["-bsf:v".to_string(), "filter_units=remove_types=8|9".to_string()]);
                    }
                    cmd.extend([
                        "-an".to_string(),
                        "-f".to_string(),
                        "mpegts".to_string(),
                        "-progress".to_string(),
                        "pipe:1".to_string(),
                        piece_path_str,
                    ]);
                }
            }

            run_ffmpeg_with_progress(
                &cmd,
                piece.duration_sec,
                seg.segment_index + 1,
                plan.segments.len(),
                base_pct,
                weight_pct,
                &stage,
                app_handle,
            )?;

            global_piece_idx += 1;
        }
    }

    // 2. Concatenar video
    let video_list_content = video_piece_files
        .iter()
        .map(|f| format!("file '{}'\n", f))
        .collect::<String>();
    let video_list_path = temp_dir.file_path("video_list.txt");
    std::fs::write(&video_list_path, video_list_content)
        .map_err(|e| format!("Error guardando lista de video: {}", e))?;

    let concat_video_path = temp_dir.file_path("video_concat.ts");
    let concat_cmd = vec![
        "-y".to_string(),
        "-fflags".to_string(),
        "+genpts".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        video_list_path.to_string_lossy().to_string(),
        "-c".to_string(),
        "copy".to_string(),
        "-avoid_negative_ts".to_string(),
        "make_zero".to_string(),
        "-progress".to_string(),
        "pipe:1".to_string(),
        concat_video_path.to_string_lossy().to_string(),
    ];

    run_ffmpeg_with_progress(
        &concat_cmd,
        plan.total_export_duration,
        plan.segments.len(),
        plan.segments.len(),
        75.0,
        10.0,
        "Ensamblando flujo de video con precisión de fotograma...",
        app_handle,
    )?;

    // 3. Procesar pistas de audio (1 corte por segmento, preservando sincronía exacta)
    let has_audio = !source.audio.is_empty();
    let total_audio_cuts = plan.audio_cuts.len();

    if has_audio {
        for (a_pos, audio_stream) in source.audio.iter().enumerate() {
            if is_cancelled() {
                return Err("Exportación cancelada".to_string());
            }

            let mut audio_seg_files = Vec::new();

            for (cut_i, cut) in plan.audio_cuts.iter().enumerate() {
                let seg_filename = format!("audio_s{}_cut_{:04}.mka", a_pos, cut_i);
                let seg_path = temp_dir.file_path(&seg_filename);
                let seg_path_str = seg_path.to_string_lossy().to_string();
                audio_seg_files.push(seg_filename);

                let mut cut_cmd = vec![
                    "-y".to_string(),
                    "-ss".to_string(),
                    format!("{:.6}", cut.start_sec),
                    "-i".to_string(),
                    source.path.clone(),
                    "-t".to_string(),
                    format!("{:.6}", cut.duration_sec),
                    "-map".to_string(),
                    format!("0:{}", audio_stream.stream_index),
                ];

                // Audio con precisión a nivel de muestra (sample-accurate) para evitar el desfase de stream-copy
                let codec = audio_stream.codec_name.to_lowercase();
                if codec == "opus" {
                    cut_cmd.extend(["-c:a".to_string(), "libopus".to_string()]);
                } else if codec == "flac" {
                    cut_cmd.extend(["-c:a".to_string(), "flac".to_string()]);
                } else {
                    cut_cmd.extend(["-c:a".to_string(), "aac".to_string()]);
                }

                if let Some(br) = audio_stream.bit_rate {
                    if br >= 32000 {
                        cut_cmd.extend(["-b:a".to_string(), format!("{}k", br / 1000)]);
                    } else {
                        cut_cmd.extend(["-b:a".to_string(), "192k".to_string()]);
                    }
                } else {
                    cut_cmd.extend(["-b:a".to_string(), "192k".to_string()]);
                }

                if audio_stream.sample_rate > 0 {
                    cut_cmd.extend(["-ar".to_string(), audio_stream.sample_rate.to_string()]);
                }
                if audio_stream.channels > 0 {
                    cut_cmd.extend(["-ac".to_string(), audio_stream.channels.to_string()]);
                }

                cut_cmd.extend([
                    "-af".to_string(),
                    "asetpts=PTS-STARTPTS".to_string(),
                    "-avoid_negative_ts".to_string(),
                    "make_zero".to_string(),
                    "-vn".to_string(),
                    "-progress".to_string(),
                    "pipe:1".to_string(),
                    seg_path_str,
                ]);

                run_ffmpeg_with_progress(
                    &cut_cmd,
                    cut.duration_sec,
                    cut_i + 1,
                    total_audio_cuts,
                    85.0,
                    5.0 / (source.audio.len() as f64),
                    &format!("Extrayendo audio con precisión de muestra pista {} ({}/{})", a_pos + 1, cut_i + 1, total_audio_cuts),
                    app_handle,
                )?;
            }

            // Concatenar audio de esta pista
            let audio_concat_path = temp_dir.file_path(&format!("audio_s{}_concat.mka", a_pos));
            if audio_seg_files.len() == 1 {
                let single_file = temp_dir.file_path(&audio_seg_files[0]);
                let _ = std::fs::copy(single_file, &audio_concat_path);
            } else {
                let list_content = audio_seg_files
                    .iter()
                    .map(|f| format!("file '{}'\n", f))
                    .collect::<String>();
                let list_file = temp_dir.file_path(&format!("audio_s{}_list.txt", a_pos));
                std::fs::write(&list_file, list_content).map_err(|e| e.to_string())?;

                let audio_concat_cmd = vec![
                    "-y".to_string(),
                    "-fflags".to_string(),
                    "+genpts".to_string(),
                    "-f".to_string(),
                    "concat".to_string(),
                    "-safe".to_string(),
                    "0".to_string(),
                    "-i".to_string(),
                    list_file.to_string_lossy().to_string(),
                    "-c".to_string(),
                    "copy".to_string(),
                    "-avoid_negative_ts".to_string(),
                    "make_zero".to_string(),
                    "-progress".to_string(),
                    "pipe:1".to_string(),
                    audio_concat_path.to_string_lossy().to_string(),
                ];

                run_ffmpeg_with_progress(
                    &audio_concat_cmd,
                    plan.total_export_duration,
                    plan.segments.len(),
                    plan.segments.len(),
                    90.0,
                    4.0,
                    &format!("Uniendo audio de la pista {}...", a_pos + 1),
                    app_handle,
                )?;
            }
        }
    }

    // 4. Multiplexado final al contenedor original
    let tmp_final_filename = format!("temp_export_{:016x}.{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), ext);
    let tmp_final_path = temp_dir.file_path(&tmp_final_filename);
    let tmp_final_path_str = tmp_final_path.to_string_lossy().to_string();

    let mut mux_cmd = vec![
        "-y".to_string(),
        "-fflags".to_string(),
        "+genpts".to_string(),
        "-i".to_string(),
        concat_video_path.to_string_lossy().to_string(),
    ];

    if has_audio {
        for a_pos in 0..source.audio.len() {
            let a_concat = temp_dir.file_path(&format!("audio_s{}_concat.mka", a_pos));
            mux_cmd.extend(["-i".to_string(), a_concat.to_string_lossy().to_string()]);
        }
    }

    // Mappings
    mux_cmd.extend(["-map".to_string(), "0:v:0".to_string()]);
    if has_audio {
        for a_pos in 0..source.audio.len() {
            mux_cmd.extend(["-map".to_string(), format!("{}:a:0", a_pos + 1)]);
        }
    }

    mux_cmd.extend([
        "-c".to_string(),
        "copy".to_string(),
        "-avoid_negative_ts".to_string(),
        "make_zero".to_string(),
    ]);

    if is_mp4 {
        mux_cmd.extend(["-movflags".to_string(), "+faststart".to_string()]);
        if let Some(ref v) = source.video {
            if let Some(ref tag) = v.codec_tag_string {
                mux_cmd.extend(["-tag:v".to_string(), tag.clone()]);
            }
        }
    }

    mux_cmd.extend([
        "-progress".to_string(),
        "pipe:1".to_string(),
        tmp_final_path_str.clone(),
    ]);

    run_ffmpeg_with_progress(
        &mux_cmd,
        plan.total_export_duration,
        plan.segments.len(),
        plan.segments.len(),
        94.0,
        5.0,
        "Generando archivo final en formato nativo...",
        app_handle,
    )?;

    // 5. Verificación de integridad post-export
    verify_output_file(&tmp_final_path_str)?;

    // 6. Mover atómicamente al destino solicitado
    if out_p.exists() {
        let _ = std::fs::remove_file(out_p);
    }
    std::fs::rename(&tmp_final_path, out_p)
        .or_else(|_| std::fs::copy(&tmp_final_path, out_p).map(|_| ()))
        .map_err(|e| format!("Error guardando archivo final en {}: {}", output_path, e))?;

    Ok(output_path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::packets::get_keyframes_with_gop_info;
    use crate::export::planner::{create_full_export_plan, normalize_segments};
    use crate::export::probe::probe_source_info;
    use crate::export::ExportSegment;

    #[test]
    fn test_smartcut_export_mp4_integration() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 1.0, end: 5.0, name: None },
            ExportSegment { start: 7.0, end: 11.0, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        let keyframes = get_keyframes_with_gop_info(path, 0, 1.0, 11.0).unwrap_or_default();
        let plan = create_full_export_plan(&source, &normalized, &keyframes);

        let temp_dir = TempDir::new("test_mp4").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_smartcut.mp4");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "Export failed: {:?}", res.err());

        let verify_res = verify_output_file(&out_str);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert!(rep.duration > 7.0 && rep.duration < 9.0);
    }

    #[test]
    fn test_smartcut_export_mkv_hevc_integration() {
        let path = "RECURSOS/videoprueba.mkv";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 2.0, end: 8.0, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        let keyframes = get_keyframes_with_gop_info(path, 0, 2.0, 8.0).unwrap_or_default();
        let plan = create_full_export_plan(&source, &normalized, &keyframes);

        let temp_dir = TempDir::new("test_mkv").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_smartcut.mkv");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "Export failed: {:?}", res.err());

        let verify_res = verify_output_file(&out_str);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert!(rep.duration > 5.5 && rep.duration < 6.5);
    }

    #[test]
    fn test_smartcut_export_multi_clip_audio_sync() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 1.2, end: 4.5, name: None },
            ExportSegment { start: 6.8, end: 10.3, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        let keyframes = get_keyframes_with_gop_info(path, 0, 1.2, 10.3).unwrap_or_default();
        let plan = create_full_export_plan(&source, &normalized, &keyframes);

        let temp_dir = TempDir::new("test_sync").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_sync.mp4");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "Export failed: {:?}", res.err());

        let verify_res = verify_output_file(&out_str);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        // Total expected duration is 3.3 + 3.5 = 6.8s
        assert!((rep.duration - 6.8).abs() < 0.25, "Expected duration close to 6.8, got {}", rep.duration);
    }
}

