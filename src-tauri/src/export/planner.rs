use serde::{Deserialize, Serialize};
use super::packets::KeyframeInfo;
use super::probe::SourceInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PieceKind {
    Encode,
    Copy,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlanPiece {
    pub kind: PieceKind,
    pub start_sec: f64,
    pub duration_sec: f64,
    pub drop_leading: bool,
    pub stage_desc: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SegmentPlan {
    pub segment_index: usize,
    pub start_sec: f64,
    pub end_sec: f64,
    pub video_pieces: Vec<PlanPiece>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioCutPlan {
    pub segment_index: usize,
    pub start_sec: f64,
    pub duration_sec: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FullExportPlan {
    pub segments: Vec<SegmentPlan>,
    pub audio_cuts: Vec<AudioCutPlan>,
    pub total_export_duration: f64,
    pub total_reencode_duration: f64,
    pub total_copy_duration: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NormalizedSegment {
    pub start: f64,
    pub end: f64,
    pub name: Option<String>,
}

pub fn normalize_segments(
    raw: &[super::ExportSegment],
    total_duration: f64,
) -> Vec<NormalizedSegment> {
    if raw.is_empty() {
        return Vec::new();
    }

    let mut valid = Vec::new();
    for s in raw {
        let st = s.start.max(0.0);
        let en = if total_duration > 0.0 {
            s.end.min(total_duration)
        } else {
            s.end
        };
        if en - st > 0.001 {
            valid.push(NormalizedSegment {
                start: st,
                end: en,
                name: s.name.clone(),
            });
        }
    }

    // VF-06: Conservar cada segmento como una instancia de edición independiente.
    // No fusionar repeticiones ni solapamientos en tiempo de fuente, garantizando
    // que las repeticiones intencionales se conserven y la duración final coincida
    // exactamente con la línea de tiempo del usuario.
    valid
}

pub fn plan_single_segment(
    seg_start: f64,
    seg_end: f64,
    fps: f64,
    keyframes: &[KeyframeInfo],
    is_hevc: bool,
) -> Vec<PlanPiece> {
    let dur = (seg_end - seg_start).max(0.001);
    let safe_fps = if fps > 0.0 { fps } else { 30.0 };
    let frame_dur = 1.0 / safe_fps;
    let half_frame = frame_dur * 0.5;

    // Segmentos breves (< 2.5s) se recodifican completos para precisión absoluta sin overhead
    if dur <= 2.5 {
        return vec![PlanPiece {
            kind: PieceKind::Encode,
            start_sec: seg_start,
            duration_sec: dur,
            drop_leading: false,
            stage_desc: "Borde de precisión (Smart Cut)".to_string(),
        }];
    }

    // Buscar keyframes que caen dentro del segmento
    let mut candidate_kfs: Vec<&KeyframeInfo> = keyframes
        .iter()
        .filter(|k| k.pts_time > seg_start + half_frame && k.pts_time < seg_end - half_frame)
        .collect();

    // En HEVC con corte cercano al inicio del archivo (seg_start < 5.0 y seg_start > 0.05),
    // el primer keyframe del archivo a menudo tiene dependencias hacia atrás en el demuxer de MKV.
    // Si disponemos de un segundo keyframe dentro del segmento, avanzar kf al segundo keyframe
    // asegura que la pieza HEAD absorba el GOP inicial y la pieza BODY empiece en un keyframe 100% limpio.
    if is_hevc && seg_start > 0.05 && candidate_kfs.len() >= 2 {
        if let Some(first_kf) = candidate_kfs.first() {
            if first_kf.pts_time < 6.0 {
                candidate_kfs.remove(0);
            }
        }
    }

    let k_first = candidate_kfs.first().copied();
    let k_last = candidate_kfs.last().copied();

    match (k_first, k_last) {
        (Some(kf), Some(kl)) if kf.pts_time < kl.pts_time => {
            let mut pieces = Vec::new();

            // 1. HEAD (re-encode borde inicial)
            let head_dur = kf.pts_time - seg_start;
            if head_dur > half_frame {
                pieces.push(PlanPiece {
                    kind: PieceKind::Encode,
                    start_sec: seg_start,
                    duration_sec: head_dur,
                    drop_leading: false,
                    stage_desc: "Borde inicial frame-accurate".to_string(),
                });
            }

            // 2. BODY (copia directa sin pérdida)
            let body_dur = kl.pts_time - kf.pts_time;
            if body_dur > half_frame {
                pieces.push(PlanPiece {
                    kind: PieceKind::Copy,
                    start_sec: kf.pts_time,
                    duration_sec: body_dur,
                    drop_leading: is_hevc && kf.is_open_gop,
                    stage_desc: "Cuerpo principal (copia sin pérdida)".to_string(),
                });
            }

            // 3. TAIL (re-encode borde final)
            let tail_dur = seg_end - kl.pts_time;
            if tail_dur > half_frame {
                pieces.push(PlanPiece {
                    kind: PieceKind::Encode,
                    start_sec: kl.pts_time,
                    duration_sec: tail_dur,
                    drop_leading: false,
                    stage_desc: "Borde final frame-accurate".to_string(),
                });
            }

            if pieces.is_empty() {
                vec![PlanPiece {
                    kind: PieceKind::Encode,
                    start_sec: seg_start,
                    duration_sec: dur,
                    drop_leading: false,
                    stage_desc: "Corte de precisión".to_string(),
                }]
            } else {
                pieces
            }
        }
        _ => {
            // Sin al menos 2 keyframes utilizables dentro del segmento, recodificar completo
            vec![PlanPiece {
                kind: PieceKind::Encode,
                start_sec: seg_start,
                duration_sec: dur,
                drop_leading: false,
                stage_desc: "Corte de precisión frame-accurate".to_string(),
            }]
        }
    }
}

pub fn create_full_export_plan(
    source: &SourceInfo,
    segments: &[NormalizedSegment],
    keyframes: &[KeyframeInfo],
) -> FullExportPlan {
    let fps = source.video.as_ref().map(|v| v.fps).unwrap_or(30.0);
    let is_hevc = source
        .video
        .as_ref()
        .map(|v| v.codec_name == "hevc" || v.codec_name == "h265")
        .unwrap_or(false);

    let mut segment_plans = Vec::new();
    let mut audio_plans = Vec::new();
    let mut total_dur = 0.0;
    let mut total_reencode = 0.0;
    let mut total_copy = 0.0;

    for (i, seg) in segments.iter().enumerate() {
        let dur = (seg.end - seg.start).max(0.001);
        total_dur += dur;

        let video_pieces = plan_single_segment(seg.start, seg.end, fps, keyframes, is_hevc);
        for p in &video_pieces {
            match p.kind {
                PieceKind::Encode => total_reencode += p.duration_sec,
                PieceKind::Copy => total_copy += p.duration_sec,
            }
        }

        segment_plans.push(SegmentPlan {
            segment_index: i,
            start_sec: seg.start,
            end_sec: seg.end,
            video_pieces,
        });

        audio_plans.push(AudioCutPlan {
            segment_index: i,
            start_sec: seg.start,
            duration_sec: dur,
        });
    }

    FullExportPlan {
        segments: segment_plans,
        audio_cuts: audio_plans,
        total_export_duration: total_dur,
        total_reencode_duration: total_reencode,
        total_copy_duration: total_copy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_clip_reencodes() {
        let pieces = plan_single_segment(1.0, 3.0, 30.0, &[], false);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].kind, PieceKind::Encode);
        assert!((pieces[0].duration_sec - 2.0).abs() < 1e-4);
    }

    #[test]
    fn test_aligned_keyframes_produces_head_body_tail() {
        let keyframes = vec![
            KeyframeInfo { pts: 0, pts_time: 0.0, is_open_gop: false, min_leading_pts: 0, min_leading_time: 0.0 },
            KeyframeInfo { pts: 60, pts_time: 2.0, is_open_gop: false, min_leading_pts: 60, min_leading_time: 2.0 },
            KeyframeInfo { pts: 120, pts_time: 4.0, is_open_gop: false, min_leading_pts: 120, min_leading_time: 4.0 },
            KeyframeInfo { pts: 180, pts_time: 6.0, is_open_gop: false, min_leading_pts: 180, min_leading_time: 6.0 },
        ];
        // Segmento de 1.0 a 5.0
        // k_first = 2.0, k_last = 4.0
        // HEAD: 1.0 a 2.0 (1.0s)
        // BODY: 2.0 a 4.0 (2.0s)
        // TAIL: 4.0 a 5.0 (1.0s)
        let pieces = plan_single_segment(1.0, 5.0, 30.0, &keyframes, false);
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[0].kind, PieceKind::Encode);
        assert_eq!(pieces[1].kind, PieceKind::Copy);
        assert_eq!(pieces[2].kind, PieceKind::Encode);
    }

    #[test]
    fn test_vf06_duplicate_and_overlapping_segments_preserved() {
        let raw = vec![
            super::super::ExportSegment { start: 1.0, end: 3.5, name: None },
            super::super::ExportSegment { start: 1.0, end: 3.5, name: None }, // Repetición exacta
            super::super::ExportSegment { start: 3.0, end: 5.0, name: None }, // Solapa el anterior
            super::super::ExportSegment { start: 10.0, end: 12.0, name: None },
        ];
        let norm = normalize_segments(&raw, 20.0);
        assert_eq!(norm.len(), 4, "VF-06: No se deben fusionar clips duplicados ni solapados");
        assert_eq!(norm[0].start, 1.0);
        assert_eq!(norm[0].end, 3.5);
        assert_eq!(norm[1].start, 1.0);
        assert_eq!(norm[1].end, 3.5);
        assert_eq!(norm[2].start, 3.0);
        assert_eq!(norm[2].end, 5.0);
        assert_eq!(norm[3].start, 10.0);
        assert_eq!(norm[3].end, 12.0);
    }

    #[test]
    fn test_reverse_order_segments_preserved() {
        let raw = vec![
            super::super::ExportSegment { start: 10.0, end: 12.0, name: None },
            super::super::ExportSegment { start: 2.0, end: 4.0, name: None },
        ];
        let norm = normalize_segments(&raw, 20.0);
        assert_eq!(norm.len(), 2);
        assert_eq!(norm[0].start, 10.0);
        assert_eq!(norm[0].end, 12.0);
        assert_eq!(norm[1].start, 2.0);
        assert_eq!(norm[1].end, 4.0);
    }
}
