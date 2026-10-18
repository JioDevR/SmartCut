use super::probe::SourceInfo;

/// VF-08: Valida combinaciones de contenedor y codec de video/audio según el modo de exportación.
/// Evita que el ensamblado falle silenciosamente o con errores crípticos de FFmpeg al intentar
/// copiar codecs no soportados por el contenedor (por ej. H.264 o HEVC en WebM, o VP8 en MP4).
pub fn validate_container_codec_compatibility(
    container: &str,
    source: &SourceInfo,
    mode: &str,
    target_codec: Option<&str>,
    audio_only: bool,
) -> Result<(), String> {
    let cont = container.to_lowercase();
    let cont = cont.trim_start_matches('.');

    if audio_only {
        match cont {
            "mp3" | "aac" | "flac" | "wav" | "m4a" | "ogg" | "mka" => return Ok(()),
            _ => {
                return Err(format!(
                    "El formato '{}' no es un contenedor de audio válido. Utilice MP3, AAC o FLAC.",
                    cont
                ));
            }
        }
    }

    let source_video_codec = source
        .video
        .as_ref()
        .map(|v| v.codec_name.to_lowercase())
        .unwrap_or_default();

    match cont {
        "mkv" => {
            // Matroska admite prácticamente cualquier codec de video y audio
            Ok(())
        }
        "webm" => {
            // WebM solo admite VP8, VP9 y AV1 para video, y Vorbis u Opus para audio
            if mode == "smart_cut" {
                if !source_video_codec.is_empty()
                    && source_video_codec != "vp8"
                    && source_video_codec != "vp9"
                    && source_video_codec != "av1"
                {
                    return Err(format!(
                        "El contenedor WebM solo es compatible con codecs de video VP8, VP9 o AV1. \
                        El video actual utiliza '{}'. Para exportar en WebM seleccione el modo \
                        'Renderizado Completo' o utilice un contenedor compatible como MKV o MP4.",
                        source_video_codec
                    ));
                }
            } else if mode == "reencode" {
                if let Some(target) = target_codec {
                    let tc = target.to_lowercase();
                    if tc == "h264" || tc == "hevc" || tc == "h265" || tc == "prores" {
                        return Err(format!(
                            "El contenedor WebM no admite el codec '{}'. Seleccione un codec compatible como VP9 o use el contenedor MP4 o MKV.",
                            target
                        ));
                    }
                }
            }
            Ok(())
        }
        "mp4" | "m4v" => {
            // MP4 admite H.264, HEVC, AV1, MPEG4
            if mode == "smart_cut" {
                if source_video_codec == "vp8" || source_video_codec == "prores" || source_video_codec == "theora" {
                    return Err(format!(
                        "El contenedor MP4 no admite copiar directamente el codec de video '{}'. \
                        Utilice el contenedor MKV o el modo 'Renderizado Completo'.",
                        source_video_codec
                    ));
                }
            } else if mode == "reencode" {
                if let Some(target) = target_codec {
                    let tc = target.to_lowercase();
                    if tc == "vp8" {
                        return Err("El contenedor MP4 no admite el codec VP8. Utilice MKV o WebM.".to_string());
                    }
                }
            }
            Ok(())
        }
        "mov" => {
            // QuickTime MOV admite H.264, HEVC, ProRes, DNxHD, AV1
            if mode == "smart_cut" && source_video_codec == "vp8" {
                return Err(format!(
                    "El contenedor MOV no admite copiar directamente el codec '{}'. Utilice MKV.",
                    source_video_codec
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::probe::{SourceInfo, VideoStreamInfo};

    fn make_test_source(codec: &str) -> SourceInfo {
        let mut s = SourceInfo::default();
        s.container_ext = "mp4".to_string();
        s.duration = 10.0;
        let mut v = VideoStreamInfo::default();
        v.codec_name = codec.to_string();
        s.video = Some(v);
        s
    }

    #[test]
    fn test_mkv_universal_compatibility() {
        let h264_src = make_test_source("h264");
        assert!(validate_container_codec_compatibility("mkv", &h264_src, "smart_cut", None, false).is_ok());

        let vp8_src = make_test_source("vp8");
        assert!(validate_container_codec_compatibility("mkv", &vp8_src, "smart_cut", None, false).is_ok());
    }

    #[test]
    fn test_webm_rejects_h264_in_smartcut() {
        let h264_src = make_test_source("h264");
        let res = validate_container_codec_compatibility("webm", &h264_src, "smart_cut", None, false);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("El contenedor WebM solo es compatible con codecs de video VP8, VP9 o AV1"));
    }

    #[test]
    fn test_webm_allows_vp9() {
        let vp9_src = make_test_source("vp9");
        assert!(validate_container_codec_compatibility("webm", &vp9_src, "smart_cut", None, false).is_ok());
    }

    #[test]
    fn test_mp4_rejects_vp8_in_smartcut() {
        let vp8_src = make_test_source("vp8");
        let res = validate_container_codec_compatibility("mp4", &vp8_src, "smart_cut", None, false);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("El contenedor MP4 no admite copiar directamente"));
    }

    #[test]
    fn test_mp4_allows_h264_and_hevc() {
        let h264_src = make_test_source("h264");
        assert!(validate_container_codec_compatibility("mp4", &h264_src, "smart_cut", None, false).is_ok());

        let hevc_src = make_test_source("hevc");
        assert!(validate_container_codec_compatibility("mp4", &hevc_src, "smart_cut", None, false).is_ok());
    }
}
