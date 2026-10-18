use std::path::Path;
use tauri::AppHandle;

use super::encoder::build_encoder_args;
use super::planner::{FullExportPlan, NormalizedSegment, PieceKind};
use super::probe::SourceInfo;
use super::runner::{is_cancelled, run_ffmpeg_with_progress};
use super::temp::TempDir;
use super::verify::verify_output_file_full;

/// VF-07: Construye argumentos de seek sample-accurate para extracción de audio.
/// Se usa seek de entrada previo como optimización únicamente si el inicio excede 10s (con 5s de margen),
/// y seek exacto posterior a `-i` para que el decodificador descarte las muestras exactas necesarias.
/// Esto elimina cualquier desfase de audio/video producido por límites de paquetes o keyframes en el contenedor.
pub fn build_sample_accurate_audio_seek_args(
    source_path: &str,
    start_sec: f64,
    duration_sec: f64,
) -> Vec<String> {
    let mut args = Vec::new();
    if start_sec > 10.0 {
        let fast_seek = (start_sec - 5.0).max(0.0);
        let exact_offset = start_sec - fast_seek;
        args.extend(["-ss".to_string(), format!("{:.6}", fast_seek)]);
        args.extend(["-i".to_string(), source_path.to_string()]);
        args.extend(["-ss".to_string(), format!("{:.6}", exact_offset)]);
    } else if start_sec > 0.0 {
        args.extend(["-i".to_string(), source_path.to_string()]);
        args.extend(["-ss".to_string(), format!("{:.6}", start_sec)]);
    } else {
        args.extend(["-i".to_string(), source_path.to_string()]);
    }
    args.extend(["-t".to_string(), format!("{:.6}", duration_sec)]);
    args
}

pub fn execute_smartcut_export_with_options(
    source: &SourceInfo,
    plan: &FullExportPlan,
    output_path: &str,
    temp_dir: &TempDir,
    hardware_accel: Option<&str>,
    crf: Option<u32>,
    preset: Option<&str>,
    video_codec: Option<&str>,
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

    // VF-11: Validar existencia de pista de video para Smart Cut
    if source.video.is_none() {
        return Err("El archivo de entrada no contiene pista de video para exportar en modo Smart Cut".to_string());
    }

    let is_mp4 = ext == "mp4" || ext == "mov" || ext == "m4v";
    let video_ordinal = source.video.as_ref().map(|v| v.video_index).unwrap_or(0);

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
                        format!("0:v:{}", video_ordinal),
                    ]);
                    cmd.extend(build_encoder_args(
                        source,
                        is_mp4,
                        hardware_accel,
                        crf,
                        preset,
                        video_codec,
                    ));
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
                    let is_mkv = source.container_ext == "mkv" || source.container_ext == "webm";

                    // En contenedores MKV con B-frames, un offset seguro (~0.18s a 0.22s) asegura que el demuxer
                    // alcance el Cluster del keyframe buscado sin retroceder al anterior.
                    let seek_offset = if is_mkv && has_b_frames {
                        (frame_dur * 12.0).clamp(0.18, 0.25)
                    } else if has_b_frames {
                        frame_dur * 4.5
                    } else {
                        frame_dur * 0.5
                    };

                    // Compensamos la duración de extracción por el offset del seek para que FFmpeg
                    // no expanda la duración total de la pieza generada
                    let effective_dur = (piece.duration_sec - seek_offset).max(frame_dur);

                    cmd.extend([
                        "-ss".to_string(),
                        format!("{:.6}", piece.start_sec + seek_offset),
                        "-i".to_string(),
                        source.path.clone(),
                        "-t".to_string(),
                        format!("{:.6}", effective_dur),
                        "-map".to_string(),
                        format!("0:v:{}", video_ordinal),
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

                let mut cut_cmd = vec!["-y".to_string()];
                cut_cmd.extend(build_sample_accurate_audio_seek_args(
                    &source.path,
                    cut.start_sec,
                    cut.duration_sec,
                ));
                cut_cmd.extend([
                    "-map".to_string(),
                    format!("0:{}", audio_stream.stream_index),
                ]);

                // Audio con precisión a nivel de muestra (sample-accurate) para evitar el desfase de stream-copy
                let codec = if ext == "webm" {
                    "opus".to_string()
                } else {
                    audio_stream.codec_name.to_lowercase()
                };
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

    // 4. Multiplexado final al contenedor solicitado
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

    // 5. Verificación de integridad y duración post-export (VF-03 & VF-10)
    let exp_audio = if has_audio { Some(source.audio.len()) } else { Some(0) };
    verify_output_file_full(
        &tmp_final_path_str,
        Some(plan.total_export_duration),
        Some(0.5),
        false,
        Some(1),
        exp_audio,
    )?;

    // 6. Mover al destino solicitado
    if out_p.exists() {
        let _ = std::fs::remove_file(out_p);
    }
    std::fs::rename(&tmp_final_path, out_p)
        .or_else(|_| std::fs::copy(&tmp_final_path, out_p).map(|_| ()))
        .map_err(|e| format!("Error guardando archivo final en {}: {}", output_path, e))?;

    Ok(output_path.to_string())
}

pub fn execute_smartcut_export(
    source: &SourceInfo,
    plan: &FullExportPlan,
    output_path: &str,
    temp_dir: &TempDir,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    execute_smartcut_export_with_options(
        source, plan, output_path, temp_dir, None, None, None, None, app_handle,
    )
}

/// VF-02: Exportación exclusiva de solo audio sin piezas ni mapas de video
pub fn execute_audio_only_export(
    source: &SourceInfo,
    segments: &[NormalizedSegment],
    output_path: &str,
    audio_format: Option<&str>,
    temp_dir: &TempDir,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    if is_cancelled() {
        return Err("Exportación cancelada".to_string());
    }

    if source.audio.is_empty() {
        return Err("El archivo de entrada no contiene pistas de audio para exportar".to_string());
    }

    let out_p = Path::new(output_path);
    if let Some(parent) = out_p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let ext = audio_format
        .map(|f| f.to_lowercase())
        .or_else(|| out_p.extension().map(|e| e.to_string_lossy().to_lowercase()))
        .unwrap_or_else(|| "mp3".to_string());

    let codec_args: Vec<String> = match ext.as_str() {
        "mp3" => vec!["-c:a".to_string(), "libmp3lame".to_string(), "-q:a".to_string(), "2".to_string()],
        "flac" => vec!["-c:a".to_string(), "flac".to_string()],
        "wav" => vec!["-c:a".to_string(), "pcm_s16le".to_string()],
        "opus" | "ogg" => vec!["-c:a".to_string(), "libopus".to_string(), "-b:a".to_string(), "128k".to_string()],
        _ => vec!["-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "192k".to_string()],
    };

    let total_duration: f64 = segments.iter().map(|s| (s.end - s.start).max(0.0)).sum();
    let audio_stream_idx = source.audio[0].stream_index;

    let tmp_final_filename = format!(
        "temp_audio_export_{:016x}.{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        ext
    );
    let tmp_final_path = temp_dir.file_path(&tmp_final_filename);
    let tmp_final_str = tmp_final_path.to_string_lossy().to_string();

    if segments.len() == 1 {
        let seg = &segments[0];
        let dur = (seg.end - seg.start).max(0.001);
        let mut cmd = vec!["-y".to_string()];
        cmd.extend(build_sample_accurate_audio_seek_args(
            &source.path,
            seg.start,
            dur,
        ));
        cmd.extend([
            "-map".to_string(),
            format!("0:{}", audio_stream_idx),
        ]);
        cmd.extend(codec_args);
        cmd.extend([
            "-vn".to_string(),
            "-avoid_negative_ts".to_string(),
            "make_zero".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            tmp_final_str.clone(),
        ]);

        run_ffmpeg_with_progress(
            &cmd,
            dur,
            1,
            1,
            0.0,
            90.0,
            "Exportando pista de audio...",
            app_handle,
        )?;
    } else {
        let mut piece_files = Vec::new();
        for (i, seg) in segments.iter().enumerate() {
            if is_cancelled() {
                return Err("Exportación cancelada".to_string());
            }

            let dur = (seg.end - seg.start).max(0.001);
            let piece_name = format!("audio_piece_{:04}.wav", i);
            let piece_path = temp_dir.file_path(&piece_name);
            let piece_str = piece_path.to_string_lossy().to_string();
            piece_files.push(piece_name);

            let base_pct = (i as f64 / segments.len() as f64) * 80.0;
            let weight_pct = 80.0 / segments.len() as f64;

            let mut cmd = vec!["-y".to_string()];
            cmd.extend(build_sample_accurate_audio_seek_args(
                &source.path,
                seg.start,
                dur,
            ));
            cmd.extend([
                "-map".to_string(),
                format!("0:{}", audio_stream_idx),
                "-c:a".to_string(),
                "pcm_s16le".to_string(),
                "-vn".to_string(),
                "-avoid_negative_ts".to_string(),
                "make_zero".to_string(),
                "-progress".to_string(),
                "pipe:1".to_string(),
                piece_str,
            ]);

            run_ffmpeg_with_progress(
                &cmd,
                dur,
                i + 1,
                segments.len(),
                base_pct,
                weight_pct,
                &format!("Extrayendo audio segmento {}/{}", i + 1, segments.len()),
                app_handle,
            )?;
        }

        let list_content = piece_files.iter().map(|f| format!("file '{}'\n", f)).collect::<String>();
        let list_path = temp_dir.file_path("audio_pieces_list.txt");
        std::fs::write(&list_path, list_content).map_err(|e| e.to_string())?;

        let mut concat_cmd = vec![
            "-y".to_string(),
            "-f".to_string(),
            "concat".to_string(),
            "-safe".to_string(),
            "0".to_string(),
            "-i".to_string(),
            list_path.to_string_lossy().to_string(),
        ];
        concat_cmd.extend(codec_args);
        concat_cmd.extend([
            "-vn".to_string(),
            "-avoid_negative_ts".to_string(),
            "make_zero".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            tmp_final_str.clone(),
        ]);

        run_ffmpeg_with_progress(
            &concat_cmd,
            total_duration,
            segments.len(),
            segments.len(),
            80.0,
            15.0,
            "Uniendo pistas de audio finales...",
            app_handle,
        )?;
    }

    verify_output_file_full(&tmp_final_str, Some(total_duration), Some(0.5), true, Some(0), Some(1))?;

    if out_p.exists() {
        let _ = std::fs::remove_file(out_p);
    }
    std::fs::rename(&tmp_final_path, out_p)
        .or_else(|_| std::fs::copy(&tmp_final_path, out_p).map(|_| ()))
        .map_err(|e| format!("Error guardando archivo de audio final en {}: {}", output_path, e))?;

    Ok(output_path.to_string())
}

/// VF-01: Renderizado completo (Reencode) con opciones de aceleración por hardware, códec, CRF y Preset
pub fn execute_full_reencode_export(
    source: &SourceInfo,
    segments: &[NormalizedSegment],
    output_path: &str,
    hardware_accel: Option<&str>,
    video_codec: Option<&str>,
    crf: Option<u32>,
    preset: Option<&str>,
    temp_dir: &TempDir,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    if is_cancelled() {
        return Err("Exportación cancelada".to_string());
    }

    if source.video.is_none() {
        return Err("El archivo de entrada no contiene pista de video para renderizar. Active la opción 'Extraer solo audio'.".to_string());
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
    let video_ordinal = source.video.as_ref().map(|v| v.video_index).unwrap_or(0);
    let total_duration: f64 = segments.iter().map(|s| (s.end - s.start).max(0.0)).sum();

    let enc_args = build_encoder_args(source, is_mp4, hardware_accel, crf, preset, video_codec);

    let tmp_final_filename = format!(
        "temp_reencode_{:016x}.{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        ext
    );
    let tmp_final_path = temp_dir.file_path(&tmp_final_filename);
    let tmp_final_str = tmp_final_path.to_string_lossy().to_string();

    if segments.len() == 1 {
        let seg = &segments[0];
        let dur = (seg.end - seg.start).max(0.001);

        let mut cmd = vec![
            "-y".to_string(),
            "-ss".to_string(),
            format!("{:.6}", seg.start),
            "-i".to_string(),
            source.path.clone(),
            "-t".to_string(),
            format!("{:.6}", dur),
            "-map".to_string(),
            format!("0:v:{}", video_ordinal),
        ];
        cmd.extend(enc_args);

        if !source.audio.is_empty() {
            for a in &source.audio {
                cmd.extend(["-map".to_string(), format!("0:{}", a.stream_index)]);
            }
            if ext == "webm" {
                cmd.extend(["-c:a".to_string(), "libopus".to_string(), "-b:a".to_string(), "128k".to_string()]);
            } else {
                cmd.extend(["-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "192k".to_string()]);
            }
        }

        if is_mp4 {
            cmd.extend(["-movflags".to_string(), "+faststart".to_string()]);
        }

        cmd.extend([
            "-avoid_negative_ts".to_string(),
            "make_zero".to_string(),
            "-progress".to_string(),
            "pipe:1".to_string(),
            tmp_final_str.clone(),
        ]);

        run_ffmpeg_with_progress(
            &cmd,
            dur,
            1,
            1,
            0.0,
            90.0,
            "Renderizado Completo con aceleración GPU/CPU...",
            app_handle,
        )?;
    } else {
        let mut video_pieces = Vec::new();
        let mut audio_pieces = Vec::new();

        for (i, seg) in segments.iter().enumerate() {
            if is_cancelled() {
                return Err("Exportación cancelada".to_string());
            }

            let dur = (seg.end - seg.start).max(0.001);
            let v_filename = format!("reencode_video_{:04}.ts", i);
            let v_path = temp_dir.file_path(&v_filename);
            let v_str = v_path.to_string_lossy().to_string();
            video_pieces.push(v_filename);

            let base_pct = (i as f64 / segments.len() as f64) * 75.0;
            let weight_pct = 75.0 / segments.len() as f64;

            let mut v_cmd = vec![
                "-y".to_string(),
                "-ss".to_string(),
                format!("{:.6}", seg.start),
                "-i".to_string(),
                source.path.clone(),
                "-t".to_string(),
                format!("{:.6}", dur),
                "-map".to_string(),
                format!("0:v:{}", video_ordinal),
            ];
            v_cmd.extend(enc_args.clone());
            v_cmd.extend([
                "-an".to_string(),
                "-f".to_string(),
                "mpegts".to_string(),
                "-progress".to_string(),
                "pipe:1".to_string(),
                v_str,
            ]);

            run_ffmpeg_with_progress(
                &v_cmd,
                dur,
                i + 1,
                segments.len(),
                base_pct,
                weight_pct,
                &format!("Renderizando segmento {}/{}...", i + 1, segments.len()),
                app_handle,
            )?;

            if !source.audio.is_empty() {
                let a_filename = format!("reencode_audio_{:04}.mka", i);
                let a_path = temp_dir.file_path(&a_filename);
                let a_str = a_path.to_string_lossy().to_string();
                audio_pieces.push(a_filename);

                let mut a_cmd = vec!["-y".to_string()];
                a_cmd.extend(build_sample_accurate_audio_seek_args(
                    &source.path,
                    seg.start,
                    dur,
                ));
                a_cmd.extend([
                    "-map".to_string(),
                    format!("0:{}", source.audio[0].stream_index),
                    "-c:a".to_string(),
                    if ext == "webm" { "libopus".to_string() } else { "aac".to_string() },
                    "-b:a".to_string(),
                    if ext == "webm" { "128k".to_string() } else { "192k".to_string() },
                    "-vn".to_string(),
                    "-avoid_negative_ts".to_string(),
                    "make_zero".to_string(),
                    "-progress".to_string(),
                    "pipe:1".to_string(),
                    a_str,
                ]);

                run_ffmpeg_with_progress(
                    &a_cmd,
                    dur,
                    i + 1,
                    segments.len(),
                    base_pct,
                    0.0,
                    &format!("Extrayendo audio segmento {}/{}...", i + 1, segments.len()),
                    app_handle,
                )?;
            }
        }

        // Concat video
        let v_list_content = video_pieces.iter().map(|f| format!("file '{}'\n", f)).collect::<String>();
        let v_list_path = temp_dir.file_path("reencode_v_list.txt");
        std::fs::write(&v_list_path, v_list_content).map_err(|e| e.to_string())?;

        let concat_v_path = temp_dir.file_path("reencode_video_concat.ts");
        let concat_v_cmd = vec![
            "-y".to_string(),
            "-f".to_string(),
            "concat".to_string(),
            "-safe".to_string(),
            "0".to_string(),
            "-i".to_string(),
            v_list_path.to_string_lossy().to_string(),
            "-c".to_string(),
            "copy".to_string(),
            "-avoid_negative_ts".to_string(),
            "make_zero".to_string(),
            concat_v_path.to_string_lossy().to_string(),
        ];
        run_ffmpeg_with_progress(
            &concat_v_cmd,
            total_duration,
            segments.len(),
            segments.len(),
            75.0,
            10.0,
            "Ensamblando video...",
            app_handle,
        )?;

        // Mux con audio
        let mut mux_cmd = vec![
            "-y".to_string(),
            "-i".to_string(),
            concat_v_path.to_string_lossy().to_string(),
        ];

        let has_audio = !audio_pieces.is_empty();
        let concat_a_path = temp_dir.file_path("reencode_audio_concat.mka");
        if has_audio {
            let a_list_content = audio_pieces.iter().map(|f| format!("file '{}'\n", f)).collect::<String>();
            let a_list_path = temp_dir.file_path("reencode_a_list.txt");
            std::fs::write(&a_list_path, a_list_content).map_err(|e| e.to_string())?;

            let concat_a_cmd = vec![
                "-y".to_string(),
                "-f".to_string(),
                "concat".to_string(),
                "-safe".to_string(),
                "0".to_string(),
                "-i".to_string(),
                a_list_path.to_string_lossy().to_string(),
                "-c".to_string(),
                "copy".to_string(),
                concat_a_path.to_string_lossy().to_string(),
            ];
            run_ffmpeg_with_progress(
                &concat_a_cmd,
                total_duration,
                segments.len(),
                segments.len(),
                85.0,
                5.0,
                "Ensamblando audio...",
                app_handle,
            )?;

            mux_cmd.extend(["-i".to_string(), concat_a_path.to_string_lossy().to_string()]);
            mux_cmd.extend(["-map".to_string(), "0:v:0".to_string(), "-map".to_string(), "1:a:0".to_string()]);
        } else {
            mux_cmd.extend(["-map".to_string(), "0:v:0".to_string()]);
        }

        mux_cmd.extend(["-c".to_string(), "copy".to_string(), "-avoid_negative_ts".to_string(), "make_zero".to_string()]);
        if is_mp4 {
            mux_cmd.extend(["-movflags".to_string(), "+faststart".to_string()]);
        }
        mux_cmd.extend(["-progress".to_string(), "pipe:1".to_string(), tmp_final_str.clone()]);

        run_ffmpeg_with_progress(
            &mux_cmd,
            total_duration,
            segments.len(),
            segments.len(),
            90.0,
            5.0,
            "Generando archivo final...",
            app_handle,
        )?;
    }

    let exp_audio = if !source.audio.is_empty() { Some(source.audio.len()) } else { Some(0) };
    verify_output_file_full(
        &tmp_final_str,
        Some(total_duration),
        Some(0.5),
        false,
        Some(1),
        exp_audio,
    )?;

    if out_p.exists() {
        let _ = std::fs::remove_file(out_p);
    }
    std::fs::rename(&tmp_final_path, out_p)
        .or_else(|_| std::fs::copy(&tmp_final_path, out_p).map(|_| ()))
        .map_err(|e| format!("Error guardando archivo reencodificado final en {}: {}", output_path, e))?;

    Ok(output_path.to_string())
}

/// VF-01: Exportar cada clip como un archivo independiente
pub fn execute_individual_clips_export(
    source: &SourceInfo,
    segments: &[NormalizedSegment],
    output_path: &str,
    audio_only: bool,
    audio_format: Option<&str>,
    hardware_accel: Option<&str>,
    video_codec: Option<&str>,
    crf: Option<u32>,
    preset: Option<&str>,
    temp_dir: &TempDir,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    if is_cancelled() {
        return Err("Exportación cancelada".to_string());
    }

    if !audio_only && source.video.is_none() {
        return Err("El archivo de entrada no contiene pista de video para exportar clips. Active la opción 'Extraer solo audio'.".to_string());
    }

    let out_p = Path::new(output_path);
    let parent_dir = out_p.parent().unwrap_or_else(|| Path::new("."));
    let stem = out_p.file_stem().and_then(|s| s.to_str()).unwrap_or("clip");
    let ext = if audio_only {
        audio_format
            .map(|f| f.to_lowercase())
            .unwrap_or_else(|| "mp3".to_string())
    } else {
        out_p.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| source.container_ext.clone())
    };

    let total = segments.len();
    let mut generated_paths = Vec::new();

    for (i, seg) in segments.iter().enumerate() {
        if is_cancelled() {
            return Err("Exportación cancelada".to_string());
        }

        let clip_name = if let Some(ref name) = seg.name {
            let clean: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
            format!("{}_{}.{}", stem, clean, ext)
        } else {
            format!("{}_clip_{:02}.{}", stem, i + 1, ext)
        };

        let clip_path = parent_dir.join(&clip_name);
        let clip_path_str = clip_path.to_string_lossy().to_string();
        let single_seg = vec![seg.clone()];

        if audio_only {
            execute_audio_only_export(
                source,
                &single_seg,
                &clip_path_str,
                Some(&ext),
                temp_dir,
                app_handle,
            )?;
        } else {
            execute_full_reencode_export(
                source,
                &single_seg,
                &clip_path_str,
                hardware_accel,
                video_codec,
                crf,
                preset,
                temp_dir,
                app_handle,
            )?;
        }

        generated_paths.push(clip_path_str);
    }

    Ok(format!("Exportados {} clips en {}", total, parent_dir.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::packets::get_keyframes_with_gop_info;
    use crate::export::planner::{create_full_export_plan, normalize_segments};
    use crate::export::probe::probe_source_info;
    use crate::export::verify::verify_output_file_detailed;
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

        let verify_res = verify_output_file_detailed(&out_str, Some(8.0), Some(0.35), false);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert!(rep.duration > 7.7 && rep.duration < 8.3);
    }

    #[test]
    fn test_vf03_copy_pieces_duration_preservation() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        // Segment 1.0 to 5.0 (4.0s total): Contains keyframes at 1.0, 2.0, 3.0, 4.0, 5.0
        let raw_segs = vec![
            ExportSegment { start: 1.0, end: 5.0, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        let keyframes = get_keyframes_with_gop_info(path, 0, 1.0, 5.0).unwrap_or_default();
        let plan = create_full_export_plan(&source, &normalized, &keyframes);

        // Verify that the plan contains a Copy piece
        let copy_piece = plan.segments[0].video_pieces.iter().find(|p| p.kind == PieceKind::Copy);
        assert!(copy_piece.is_some(), "Plan must contain a Copy piece to test duration preservation");
        let copy_dur = copy_piece.unwrap().duration_sec;
        assert!(copy_dur >= 1.0, "Copy piece duration must be >= 1.0s");

        let temp_dir = TempDir::new("test_vf03").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_vf03.mp4");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "SmartCut export failed: {:?}", res.err());

        // Verificamos que la duración real no se acorta y coincide exactamente con 4.0s (tolerancia <= 0.20s)
        let rep = verify_output_file_detailed(&out_str, Some(4.0), Some(0.20), false).expect("Verification failed");
        assert!((rep.duration - 4.0).abs() < 0.20, "Output duration {} must be within 0.20s of 4.0s expected", rep.duration);
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

        let verify_res = verify_output_file_detailed(&out_str, Some(6.0), Some(0.5), false);
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

        let verify_res = verify_output_file_detailed(&out_str, Some(6.8), Some(0.35), false);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert!((rep.duration - 6.8).abs() < 0.35, "Expected duration close to 6.8, got {}", rep.duration);
    }

    #[test]
    fn test_full_reencode_export_integration() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 1.0, end: 4.0, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);

        let temp_dir = TempDir::new("test_reencode").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_reencode.mp4");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_full_reencode_export(
            &source,
            &normalized,
            &out_str,
            Some("cpu"),
            Some("h264"),
            Some(22),
            Some("ultrafast"),
            &temp_dir,
            None,
        );
        assert!(res.is_ok(), "Reencode failed: {:?}", res.err());

        let verify_res = verify_output_file_detailed(&out_str, Some(3.0), Some(0.35), false);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert!((rep.duration - 3.0).abs() < 0.25);
        assert!(rep.video_streams >= 1);
    }

    #[test]
    fn test_audio_only_export_mp3_integration() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 1.0, end: 4.0, name: None },
            ExportSegment { start: 6.0, end: 8.0, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);

        let temp_dir = TempDir::new("test_audio_mp3").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_audio.mp3");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_audio_only_export(
            &source,
            &normalized,
            &out_str,
            Some("mp3"),
            &temp_dir,
            None,
        );
        assert!(res.is_ok(), "Audio export failed: {:?}", res.err());

        let verify_res = verify_output_file_detailed(&out_str, Some(5.0), Some(0.35), true);
        assert!(verify_res.is_ok(), "Audio verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert_eq!(rep.video_streams, 0, "Audio-only output must contain 0 video streams");
        assert!(rep.audio_streams >= 1, "Audio-only output must contain at least 1 audio stream");
        assert!((rep.duration - 5.0).abs() < 0.25);
    }

    #[test]
    fn test_audio_only_export_aac_integration() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 2.0, end: 5.5, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);

        let temp_dir = TempDir::new("test_audio_aac").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_audio.m4a");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_audio_only_export(
            &source,
            &normalized,
            &out_str,
            Some("aac"),
            &temp_dir,
            None,
        );
        assert!(res.is_ok(), "Audio AAC export failed: {:?}", res.err());

        let verify_res = verify_output_file_detailed(&out_str, Some(3.5), Some(0.35), true);
        assert!(verify_res.is_ok(), "Audio AAC verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert_eq!(rep.video_streams, 0);
        assert!(rep.audio_streams >= 1);
        assert!((rep.duration - 3.5).abs() < 0.25);
    }

    #[test]
    fn test_audio_only_export_flac_integration() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 1.0, end: 3.5, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);

        let temp_dir = TempDir::new("test_audio_flac").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_audio.flac");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_audio_only_export(
            &source,
            &normalized,
            &out_str,
            Some("flac"),
            &temp_dir,
            None,
        );
        assert!(res.is_ok(), "Audio FLAC export failed: {:?}", res.err());

        let verify_res = verify_output_file_detailed(&out_str, Some(2.5), Some(0.35), true);
        assert!(verify_res.is_ok(), "Audio FLAC verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert_eq!(rep.video_streams, 0);
        assert!(rep.audio_streams >= 1);
        assert!((rep.duration - 2.5).abs() < 0.25);
    }

    #[test]
    fn test_audio_only_without_audio_fails() {
        let mut source = SourceInfo::default();
        source.path = "dummy.mp4".to_string();
        source.audio = vec![]; // Sin audio

        let normalized = vec![NormalizedSegment { start: 0.0, end: 5.0, name: None }];
        let temp_dir = TempDir::new("test_no_audio").expect("temp dir failed");
        let out_path = temp_dir.file_path("out.mp3");

        let res = execute_audio_only_export(
            &source,
            &normalized,
            &out_path.to_string_lossy(),
            Some("mp3"),
            &temp_dir,
            None,
        );

        assert!(res.is_err(), "Must fail when no audio stream exists");
        assert!(res.unwrap_err().contains("no contiene pistas de audio"));
    }

    #[test]
    fn test_individual_clips_export_integration() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 1.0, end: 3.0, name: Some("partA".to_string()) },
            ExportSegment { start: 5.0, end: 7.0, name: Some("partB".to_string()) },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);

        let temp_dir = TempDir::new("test_individual").expect("temp dir failed");
        let out_path = temp_dir.file_path("output_video.mp4");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_individual_clips_export(
            &source,
            &normalized,
            &out_str,
            false,
            None,
            Some("cpu"),
            Some("h264"),
            Some(22),
            Some("ultrafast"),
            &temp_dir,
            None,
        );
        assert!(res.is_ok(), "Individual clips export failed: {:?}", res.err());

        let clip1 = temp_dir.file_path("output_video_partA.mp4");
        let clip2 = temp_dir.file_path("output_video_partB.mp4");
        assert!(clip1.exists(), "Clip 1 should exist");
        assert!(clip2.exists(), "Clip 2 should exist");

        let v1 = verify_output_file_detailed(&clip1.to_string_lossy(), Some(2.0), Some(0.35), false);
        assert!(v1.is_ok(), "Clip 1 verification failed: {:?}", v1.err());

        let v2 = verify_output_file_detailed(&clip2.to_string_lossy(), Some(2.0), Some(0.35), false);
        assert!(v2.is_ok(), "Clip 2 verification failed: {:?}", v2.err());
    }

    #[test]
    fn test_vf04_audio_before_video_stream_handling() {
        let temp_dir = TempDir::new("test_vf04").expect("temp dir failed");
        let fixture_path = temp_dir.file_path("audio_first.mkv");
        let fixture_str = fixture_path.to_string_lossy().to_string();

        // 1. Generar archivo donde Stream 0 es Audio y Stream 1 es Video
        let gen_status = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "lavfi", "-i", "aevalsrc=0:d=4",
                "-f", "lavfi", "-i", "testsrc=duration=4:size=320x240:rate=30",
                "-map", "0:a:0",
                "-map", "1:v:0",
                "-c:a", "aac",
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                &fixture_str,
            ])
            .output();

        if gen_status.is_err() || !gen_status.unwrap().status.success() {
            // Si ffmpeg no está disponible en este entorno para lavfi, salir
            return;
        }

        // 2. Verificar que probe_source_info identifique correctamente ordinal vs índice global
        let source = probe_source_info(&fixture_str).expect("probe failed");
        let v = source.video.as_ref().expect("Must find video");
        assert_eq!(v.stream_index, 1, "Global stream index for video must be 1");
        assert_eq!(v.video_index, 0, "Video ordinal within video streams must be 0");

        assert_eq!(source.audio.len(), 1);
        assert_eq!(source.audio[0].stream_index, 0, "Global stream index for audio must be 0");
        assert_eq!(source.audio[0].audio_index, 0, "Audio ordinal must be 0");

        // 3. get_keyframes_with_gop_info usando video_index (ordinal 0)
        let keyframes = get_keyframes_with_gop_info(&fixture_str, v.video_index, 0.0, 4.0)
            .expect("keyframes query with video ordinal must succeed");
        assert!(!keyframes.is_empty(), "Must find keyframes using video ordinal");

        // 4. Probar exportación Smart Cut sobre este archivo con stream global 1
        let raw_segs = vec![ExportSegment { start: 0.5, end: 2.5, name: None }];
        let normalized = normalize_segments(&raw_segs, source.duration);
        let plan = create_full_export_plan(&source, &normalized, &keyframes);

        let out_path = temp_dir.file_path("out_audio_first.mkv");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "Smart Cut export on audio-first file must succeed: {:?}", res.err());

        let verify_res = verify_output_file_detailed(&out_str, Some(2.0), Some(0.35), false);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert_eq!(rep.video_streams, 1);
        assert_eq!(rep.audio_streams, 1);
    }

    #[test]
    fn test_vf05_non_monotonic_clips_export() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        // Clips en orden inverso de fuente: primero tramo [7.0, 10.0] (3s), luego [1.0, 3.5] (2.5s)
        let raw_segs = vec![
            ExportSegment { start: 7.0, end: 10.0, name: None },
            ExportSegment { start: 1.0, end: 3.5, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].start, 7.0);
        assert_eq!(normalized[1].start, 1.0);

        // VF-05: El rango de keyframes se debe calcular con min(start) y max(end) globales
        let min_start = normalized.iter().map(|s| s.start).fold(f64::INFINITY, f64::min).max(0.0).min(source.duration);
        let max_end = normalized.iter().map(|s| s.end).fold(0.0_f64, f64::max).min(source.duration);
        assert_eq!(min_start, 1.0, "min_start must be 1.0");
        assert_eq!(max_end, 10.0, "max_end must be 10.0");

        let video_ordinal = source.video.as_ref().map(|v| v.video_index).unwrap_or(0);
        let keyframes = get_keyframes_with_gop_info(path, video_ordinal, min_start, max_end)
            .expect("keyframes query with min/max must succeed");
        assert!(!keyframes.is_empty(), "Keyframes list must not be empty");

        let plan = create_full_export_plan(&source, &normalized, &keyframes);
        assert_eq!(plan.segments.len(), 2);
        assert_eq!(plan.segments[0].start_sec, 7.0);
        assert_eq!(plan.segments[1].start_sec, 1.0);
        // Duración total esperada: 3.0s + 2.5s = 5.5s
        assert!((plan.total_export_duration - 5.5).abs() < 1e-3);

        let temp_dir = TempDir::new("test_vf05").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_non_monotonic.mp4");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "Export with reverse source order must succeed: {:?}", res.err());

        let verify_res = verify_output_file_detailed(&out_str, Some(5.5), Some(0.35), false);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert!((rep.duration - 5.5).abs() < 0.35);
    }

    #[test]
    fn test_vf06_duplicate_range_clips_export() {
        let path = "RECURSOS/videoprueba.mp4";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        // VF-06: Dos clips idénticos que repiten exactamente el mismo rango de fuente: [1.0, 3.0] (2s cada uno)
        let raw_segs = vec![
            ExportSegment { start: 1.0, end: 3.0, name: None },
            ExportSegment { start: 1.0, end: 3.0, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        assert_eq!(normalized.len(), 2, "VF-06: Los clips duplicados no deben fusionarse");
        assert_eq!(normalized[0].start, 1.0);
        assert_eq!(normalized[0].end, 3.0);
        assert_eq!(normalized[1].start, 1.0);
        assert_eq!(normalized[1].end, 3.0);

        let video_ordinal = source.video.as_ref().map(|v| v.video_index).unwrap_or(0);
        let min_start = 1.0;
        let max_end = 3.0;

        let keyframes = get_keyframes_with_gop_info(path, video_ordinal, min_start, max_end)
            .expect("keyframes query must succeed");

        let plan = create_full_export_plan(&source, &normalized, &keyframes);
        assert_eq!(plan.segments.len(), 2);
        // Duración total esperada: 2.0s + 2.0s = 4.0s
        assert!((plan.total_export_duration - 4.0).abs() < 1e-3);

        let temp_dir = TempDir::new("test_vf06").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_duplicate_clips.mp4");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "Export with repeated clip range must succeed: {:?}", res.err());

        let verify_res = verify_output_file_detailed(&out_str, Some(4.0), Some(0.35), false);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
        let rep = verify_res.unwrap();
        assert!((rep.duration - 4.0).abs() < 0.35);
    }

    #[test]
    fn test_vf07_sample_accurate_audio_seeking_and_av_sync() {
        // 1. Validar que la función de generación de argumentos de seek construya la estrategia adecuada
        let args_short = build_sample_accurate_audio_seek_args("test.mp4", 4.25, 2.0);
        assert_eq!(args_short, vec!["-i", "test.mp4", "-ss", "4.250000", "-t", "2.000000"]);

        let args_zero = build_sample_accurate_audio_seek_args("test.mp4", 0.0, 3.0);
        assert_eq!(args_zero, vec!["-i", "test.mp4", "-t", "3.000000"]);

        let args_long = build_sample_accurate_audio_seek_args("test.mp4", 25.0, 4.0);
        assert_eq!(
            args_long,
            vec!["-ss", "20.000000", "-i", "test.mp4", "-ss", "5.000000", "-t", "4.000000"]
        );

        // 2. Probar exportación Smart Cut sobre videoprueba.mkv comparando sincronía y timestamps
        let path = "RECURSOS/videoprueba.mkv";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 2.35, end: 5.75, name: None }, // 3.40s
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        let video_ordinal = source.video.as_ref().map(|v| v.video_index).unwrap_or(0);
        let keyframes = get_keyframes_with_gop_info(path, video_ordinal, 2.35, 5.75)
            .expect("keyframes failed");

        let plan = create_full_export_plan(&source, &normalized, &keyframes);
        let temp_dir = TempDir::new("test_vf07").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_vf07_sync.mkv");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "Export failed: {:?}", res.err());

        // Verificar con ffprobe que el primer timestamp de video y audio no tengan desfase
        let probe_v = std::process::Command::new("ffprobe")
            .args([
                "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "packet=pts_time",
                "-of", "csv=p=0",
                &out_str,
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        let first_v_pts: f64 = probe_v.lines().next().and_then(|l| l.trim().parse().ok()).unwrap_or(0.0);

        let probe_a = std::process::Command::new("ffprobe")
            .args([
                "-v", "error",
                "-select_streams", "a:0",
                "-show_entries", "packet=pts_time",
                "-of", "csv=p=0",
                &out_str,
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        let first_a_pts: f64 = probe_a.lines().next().and_then(|l| l.trim().parse().ok()).unwrap_or(0.0);

        // El desfase entre el primer frame de video y el primer paquete de audio debe ser menor a 80ms (tolerancia de 2 frames a 30fps por B-frames reordering)
        let sync_delta = (first_v_pts - first_a_pts).abs();
        assert!(
            sync_delta < 0.08,
            "Desfase A/V excesivo: video start {:.4}s, audio start {:.4}s (delta {:.4}s)",
            first_v_pts, first_a_pts, sync_delta
        );

        let verify_res = verify_output_file_detailed(&out_str, Some(3.40), Some(0.35), false);
        assert!(verify_res.is_ok(), "Verification failed: {:?}", verify_res.err());
    }

    #[test]
    fn test_smartcut_hevc_early_cut_calibration() {
        let path = "/home/jiodev/Videos/2026-10-13 13-06-55.mkv";
        if !std::path::Path::new(path).exists() {
            return;
        }

        let source = probe_source_info(path).expect("probe failed");
        let raw_segs = vec![
            ExportSegment { start: 2.5, end: 20.0, name: None },
        ];
        let normalized = normalize_segments(&raw_segs, source.duration);
        let keyframes = get_keyframes_with_gop_info(path, 0, 0.0, 25.0)
            .expect("keyframes failed");

        let plan = create_full_export_plan(&source, &normalized, &keyframes);
        assert_eq!(plan.total_export_duration, 17.5);

        let temp_dir = TempDir::new("test_early_hevc").expect("temp dir failed");
        let out_path = temp_dir.file_path("out_early_hevc.mkv");
        let out_str = out_path.to_string_lossy().to_string();

        let res = execute_smartcut_export(&source, &plan, &out_str, &temp_dir, None);
        assert!(res.is_ok(), "SmartCut export for early cut HEVC failed: {:?}", res.err());

        // Verificamos que la salida pase la comprobación estricta VF-10 (tolerancia 0.35s)
        let verify_res = verify_output_file_detailed(&out_str, Some(17.5), Some(0.35), false);
        assert!(verify_res.is_ok(), "VF-10 verification must succeed: {:?}", verify_res.err());
    }
}
