use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Mutex, OnceLock};
use tiny_http::{Header, Response, Server, StatusCode};

pub static STREAM_SERVER_PORT: AtomicU16 = AtomicU16::new(0);
static STREAM_SESSION_TOKEN: OnceLock<String> = OnceLock::new();
static ALLOWED_STREAM_PATHS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

pub fn get_or_init_session_token() -> &'static str {
    STREAM_SESSION_TOKEN.get_or_init(|| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(123456789);
        let pid = std::process::id();
        let mut hasher = DefaultHasher::new();
        now.hash(&mut hasher);
        pid.hash(&mut hasher);
        format!("{:016x}{:016x}", hasher.finish(), now)
    })
}

pub fn authorize_stream_path(path: &str) {
    if let Ok(mut lock) = ALLOWED_STREAM_PATHS.lock() {
        let set = lock.get_or_insert_with(HashSet::new);
        set.insert(path.to_string());
        if let Ok(canon) = std::fs::canonicalize(path) {
            set.insert(canon.to_string_lossy().to_string());
        }
    }
}

pub fn is_stream_path_authorized(path: &str) -> bool {
    if let Ok(lock) = ALLOWED_STREAM_PATHS.lock() {
        if let Some(set) = lock.as_ref() {
            if set.contains(path) {
                return true;
            }
            if let Ok(canon) = std::fs::canonicalize(path) {
                if set.contains(&canon.to_string_lossy().to_string()) {
                    return true;
                }
            }
        }
    }
    let temp_dir = std::env::temp_dir();
    let p = Path::new(path);
    if let Ok(canon_p) = std::fs::canonicalize(p) {
        if let Ok(canon_temp) = std::fs::canonicalize(&temp_dir) {
            if canon_p.starts_with(&canon_temp) {
                if let Some(file_name) = canon_p.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("smartcut_preview_")
                        || file_name.starts_with("smartcut_thumb_")
                        || file_name.starts_with("smartcut_audio_")
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn urlencoding_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..=i + 2]).unwrap_or(""), 16)
            {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' || b == b'/' {
            result.push(b as char);
        } else {
            result.push_str(&format!("%{:02X}", b));
        }
    }
    result
}

#[derive(Debug, PartialEq, Eq)]
pub enum RangeSpec {
    Satisfiable(u64, u64),
    Unsatisfiable,
    None,
}

pub fn parse_range_header(header: &str, total_len: u64) -> RangeSpec {
    if total_len == 0 {
        return RangeSpec::Unsatisfiable;
    }
    let s = header.trim();
    if !s.starts_with("bytes=") {
        return RangeSpec::None;
    }
    let range_part = s[6..].trim();
    let Some((start_str, end_str)) = range_part.split_once('-') else {
        return RangeSpec::None;
    };
    let start_str = start_str.trim();
    let end_str = end_str.trim();

    if start_str.is_empty() {
        if let Ok(suffix_len) = end_str.parse::<u64>() {
            if suffix_len == 0 {
                return RangeSpec::Unsatisfiable;
            }
            let start = total_len.saturating_sub(suffix_len);
            let end = total_len - 1;
            RangeSpec::Satisfiable(start, end)
        } else {
            RangeSpec::None
        }
    } else {
        let Ok(start) = start_str.parse::<u64>() else {
            return RangeSpec::None;
        };
        if start >= total_len {
            return RangeSpec::Unsatisfiable;
        }
        let end = if end_str.is_empty() {
            total_len - 1
        } else if let Ok(parsed_end) = end_str.parse::<u64>() {
            parsed_end.min(total_len - 1)
        } else {
            return RangeSpec::None;
        };

        if start <= end {
            RangeSpec::Satisfiable(start, end)
        } else {
            RangeSpec::Unsatisfiable
        }
    }
}

pub fn start_streaming_server() {
    let server = match Server::http("127.0.0.1:0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error iniciando servidor de streaming: {}", e);
            return;
        }
    };

    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .unwrap_or(0);

    STREAM_SERVER_PORT.store(port, Ordering::SeqCst);
    println!("Servidor de streaming SmartCut activo en http://127.0.0.1:{}/", port);

    let server = std::sync::Arc::new(server);
    let worker_count = 8;
    for _ in 0..worker_count {
        let s = server.clone();
        std::thread::spawn(move || {
            while let Ok(request) = s.recv() {
                handle_stream_request(request);
            }
        });
    }
}

fn handle_stream_request(request: tiny_http::Request) {
    let url = request.url().to_string();

    let mut token_val = None;
    if let Some(pos) = url.find("token=") {
        let raw = &url[pos + 6..];
        let end_pos = raw.find('&').unwrap_or(raw.len());
        token_val = Some(&raw[..end_pos]);
    }

    let mut file_path = None;
    if let Some(pos) = url.find("path=") {
        let raw = &url[pos + 5..];
        let end_pos = raw.find('&').unwrap_or(raw.len());
        let slice = &raw[..end_pos];
        file_path = Some(urlencoding_decode(slice));
    }

    let valid_token = get_or_init_session_token();
    let token_ok = token_val.map(|t| t == valid_token).unwrap_or(false);

    if !token_ok {
        let _ = request.respond(Response::empty(StatusCode(403)));
        return;
    }

    let Some(path_str) = file_path else {
        let _ = request.respond(Response::empty(StatusCode(400)));
        return;
    };

    if !is_stream_path_authorized(&path_str) {
        let _ = request.respond(Response::empty(StatusCode(403)));
        return;
    }

    let path = PathBuf::from(&path_str);
    if !path.exists() {
        let _ = request.respond(Response::empty(StatusCode(404)));
        return;
    }

    let Ok(mut file) = File::open(&path) else {
        let _ = request.respond(Response::empty(StatusCode(500)));
        return;
    };

    let total_len = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => {
            let _ = request.respond(Response::empty(StatusCode(500)));
            return;
        }
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime_type = match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        _ => "video/mp4",
    };

    if *request.method() == tiny_http::Method::Head {
        let resp = Response::new(
            StatusCode(200),
            vec![
                Header::from_bytes(&b"Content-Type"[..], mime_type.as_bytes()).unwrap(),
                Header::from_bytes(&b"Accept-Ranges"[..], &b"bytes"[..]).unwrap(),
                Header::from_bytes(&b"Content-Length"[..], total_len.to_string().as_bytes()).unwrap(),
                Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
            ],
            std::io::empty(),
            Some(0),
            None,
        )
        .with_chunked_threshold(usize::MAX);
        let _ = request.respond(resp);
        return;
    }

    let range_header = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("range"))
        .map(|h| h.value.as_str().to_string());

    let range_spec = match range_header {
        Some(ref rh) => parse_range_header(rh, total_len),
        None => RangeSpec::None,
    };

    match range_spec {
        RangeSpec::Unsatisfiable => {
            let resp = Response::new(
                StatusCode(416),
                vec![
                    Header::from_bytes(&b"Content-Range"[..], format!("bytes */{}", total_len).as_bytes()).unwrap(),
                    Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                ],
                std::io::empty(),
                Some(0),
                None,
            )
            .with_chunked_threshold(usize::MAX);
            let _ = request.respond(resp);
        }
        RangeSpec::Satisfiable(start, end) => {
            // Para archivos Matroska (MKV/WebM), GStreamer matroska-demux requiere leer clusters completos
            // y asume el rango abierto; truncar a fragmentos pequeños causa assertion failed en search_cluster.
            // Para otros formatos (MP4/H.264), se limita a un fragmento máximo (ej. 8-16 MiB) para optimizar scrubbing.
            let chunk_limit = if ext == "mkv" || ext == "webm" {
                total_len
            } else {
                16 * 1024 * 1024
            };
            let end = end.min(start + chunk_limit - 1);
            let chunk_len = end - start + 1;
            if file.seek(SeekFrom::Start(start)).is_err() {
                let _ = request.respond(Response::empty(StatusCode(500)));
                return;
            }

            let limited_reader = file.take(chunk_len);
            let content_range = format!("bytes {}-{}/{}", start, end, total_len);

            let resp = Response::new(
                StatusCode(206),
                vec![
                    Header::from_bytes(&b"Content-Type"[..], mime_type.as_bytes()).unwrap(),
                    Header::from_bytes(&b"Content-Range"[..], content_range.as_bytes()).unwrap(),
                    Header::from_bytes(&b"Accept-Ranges"[..], &b"bytes"[..]).unwrap(),
                    Header::from_bytes(&b"Content-Length"[..], chunk_len.to_string().as_bytes()).unwrap(),
                    Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    Header::from_bytes(&b"Access-Control-Expose-Headers"[..], &b"Content-Range, Content-Length, Accept-Ranges"[..]).unwrap(),
                ],
                limited_reader,
                Some(chunk_len as usize),
                None,
            )
            .with_chunked_threshold(usize::MAX);
            let _ = request.respond(resp);
        }
        RangeSpec::None => {
            let resp = Response::new(
                StatusCode(200),
                vec![
                    Header::from_bytes(&b"Content-Type"[..], mime_type.as_bytes()).unwrap(),
                    Header::from_bytes(&b"Accept-Ranges"[..], &b"bytes"[..]).unwrap(),
                    Header::from_bytes(&b"Content-Length"[..], total_len.to_string().as_bytes()).unwrap(),
                    Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
                    Header::from_bytes(&b"Access-Control-Expose-Headers"[..], &b"Content-Length, Accept-Ranges"[..]).unwrap(),
                ],
                file,
                Some(total_len as usize),
                None,
            )
            .with_chunked_threshold(usize::MAX);
            let _ = request.respond(resp);
        }
    }
}

#[tauri::command]
pub fn get_video_stream_url(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("El archivo no existe: {}", path));
    }

    let port = STREAM_SERVER_PORT.load(Ordering::SeqCst);
    if port == 0 {
        return Err("Servidor de streaming no inicializado".to_string());
    }

    authorize_stream_path(&path);
    let token = get_or_init_session_token();
    let encoded_path = urlencoding_encode(&path);
    Ok(format!("http://127.0.0.1:{}/video?token={}&path={}", port, token, encoded_path))
}
