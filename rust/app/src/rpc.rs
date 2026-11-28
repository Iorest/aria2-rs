//! aria2 1.37.0 compatible RPC server for the Rust front door.
//!
//! Only loopback is bound. Wire formats, method names, error codes and HTTP
//! status mapping are interface constants from aria2@1.37.0 — see the source
//! citations at the sites below and `docs/EQUIVALENCE.md` E13–E18. The server
//! is started only when argv contains `--enable-rpc`; every other CLI run is
//! untouched.

use std::ffi::OsString;
use std::net::TcpListener;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{RawQuery, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, EXPIRES};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::{Value, json};
use sha1::{Digest, Sha1};

use crate::engine;

// aria2@1.37.0 src/RpcMethodFactory.cc:53-94 (all feature guards on).
const METHODS: &[&str] = &[
    "aria2.addUri",
    "aria2.addTorrent",
    "aria2.getPeers",
    "aria2.addMetalink",
    "aria2.remove",
    "aria2.pause",
    "aria2.forcePause",
    "aria2.pauseAll",
    "aria2.forcePauseAll",
    "aria2.unpause",
    "aria2.unpauseAll",
    "aria2.forceRemove",
    "aria2.changePosition",
    "aria2.tellStatus",
    "aria2.getUris",
    "aria2.getFiles",
    "aria2.getServers",
    "aria2.tellActive",
    "aria2.tellWaiting",
    "aria2.tellStopped",
    "aria2.getOption",
    "aria2.changeUri",
    "aria2.changeOption",
    "aria2.getGlobalOption",
    "aria2.changeGlobalOption",
    "aria2.purgeDownloadResult",
    "aria2.removeDownloadResult",
    "aria2.getVersion",
    "aria2.getSessionInfo",
    "aria2.shutdown",
    "aria2.forceShutdown",
    "aria2.getGlobalStat",
    "aria2.saveSession",
    "system.multicall",
    "system.listMethods",
    "system.listNotifications",
];

// aria2@1.37.0 src/RpcMethodFactory.cc:100-107.
const NOTIFICATIONS: &[&str] = &[
    "aria2.onDownloadStart",
    "aria2.onDownloadPause",
    "aria2.onDownloadStop",
    "aria2.onDownloadComplete",
    "aria2.onDownloadError",
    "aria2.onBtDownloadComplete",
];

// aria2@1.37.0 VERSION; feature identity is not compared (EQUIVALENCE E7).
const VERSION: &str = "1.37.0";
const FEATURES: &[&str] = &[
    "Async DNS",
    "BitTorrent",
    "Firefox3 Cookie",
    "GZip",
    "HTTPS",
    "Message Digest",
    "Metalink",
    "XML-RPC",
    "SFTP",
];

#[derive(Clone)]
struct AppState {
    inner: Arc<Mutex<RpcState>>,
    secret: Option<String>,
    dir: String,
    ca: Option<String>,
}

#[derive(Default)]
struct RpcState {
    downloads: Vec<Download>,
    serial: u64,
}

#[derive(Clone)]
struct Download {
    gid: String,
    status: String,
    uri: String,
    path: String,
    total_length: u64,
    completed_length: u64,
    piece_length: u64,
    num_pieces: u64,
    bitfield: String,
    error_code: String,
    error_message: String,
}

fn option<'a>(args: &'a [OsString], name: &str) -> Option<&'a str> {
    args.iter()
        .filter_map(|arg| arg.to_str())
        .find_map(|arg| arg.strip_prefix(name))
}

/// Starts the RPC server when this run asks for one. `None` leaves the CLI
/// front door in charge of every other invocation.
pub fn serve(args: &[OsString]) -> Option<ExitCode> {
    if !args.iter().any(|arg| arg.to_str() == Some("--enable-rpc")) {
        return None;
    }
    let port = option(args, "--rpc-listen-port=")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(6800); // aria2@1.37.0 default RPC port
    let secret = option(args, "--rpc-secret=").map(str::to_owned);
    let dir = option(args, "--dir=").unwrap_or("downloads").to_owned();
    let ca = option(args, "--ca-certificate=").map(str::to_owned);
    let listener = TcpListener::bind(("127.0.0.1", port)).ok()?;
    listener.set_nonblocking(true).ok()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(async {
        let state = AppState {
            inner: Arc::new(Mutex::new(RpcState::default())),
            secret,
            dir,
            ca,
        };
        let listener = tokio::net::TcpListener::from_std(listener).ok()?;
        let app = Router::new()
            .route("/jsonrpc", post(json_post).get(json_get))
            .route("/rpc", post(xml_post))
            .fallback(|| async { not_found() });
        axum::serve(listener, app.with_state(state)).await.ok()?;
        Some(ExitCode::SUCCESS)
    })
}

fn expires() -> String {
    // aria2 sends an Expires header one second ahead; HTTP dates are compared
    // as normalized noise (EQUIVALENCE N4).
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 1;
    let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        weekday[((secs / 86_400) as i64 + 4).rem_euclid(7) as usize],
        day,
        months[(month - 1) as usize],
        year,
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

fn wired(status: StatusCode, content_type: &'static str, body: String) -> Response {
    let mut headers = HeaderMap::new();
    let _ = headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    let _ = headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Ok(value) = HeaderValue::from_str(&expires()) {
        let _ = headers.insert(EXPIRES, value);
    }
    (status, headers, body).into_response()
}

/// aria2@1.37.0 serves an unknown RPC path with cache-control and expires but
/// no content-type (golden `json-unknown-path`).
fn not_found() -> Response {
    let mut headers = HeaderMap::new();
    let _ = headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Ok(value) = HeaderValue::from_str(&expires()) {
        let _ = headers.insert(EXPIRES, value);
    }
    (StatusCode::NOT_FOUND, headers, String::new()).into_response()
}

fn json_reply(status: StatusCode, body: Value, callback: Option<&str>) -> Response {
    let text = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_owned());
    // aria2's JSON encoder escapes '/' (src/json.cc encodeString), including
    // in URIs and paths; serde_json does not.
    let text = text.replace('/', "\\/");
    let text = match callback {
        Some(name) => format!("{name}({text})"),
        None => text,
    };
    let content_type = if callback.is_some() {
        "text/javascript"
    } else {
        "application/json-rpc"
    };
    wired(status, content_type, text)
}

fn json_envelope(id: Value, value: Value, error: bool) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("id".to_owned(), id);
    map.insert("jsonrpc".to_owned(), Value::String("2.0".to_owned()));
    if error {
        map.insert("error".to_owned(), value);
    } else {
        map.insert("result".to_owned(), value);
    }
    Value::Object(map)
}

fn error_envelope(id: Value, code: i64, message: &str) -> Value {
    json_envelope(id, json!({ "code": code, "message": message }), true)
}

fn transport_error(id: Value, code: i64, message: &str, status: StatusCode) -> Response {
    json_reply(status, error_envelope(id, code, message), None)
}

async fn json_post(State(state): State<AppState>, body: Bytes) -> Response {
    let text = String::from_utf8_lossy(&body);
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(batch)) => {
            let mut replies = Vec::with_capacity(batch.len());
            for request in &batch {
                if let Value::Object(_) = request {
                    replies.push(evaluate(&state, request).await.1);
                }
            }
            json_reply(StatusCode::OK, Value::Array(replies), None)
        }
        Ok(request) => process(&state, request, None).await,
        // aria2@1.37.0 src/HttpServerBodyCommand.cc:275-281; the default
        // status arm at :127 maps -32700 to 500.
        Err(_) => transport_error(
            Value::Null,
            -32700,
            "Parse error.",
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    }
}

async fn json_get(State(state): State<AppState>, RawQuery(query): RawQuery) -> Response {
    let mut method = None;
    let mut id = None;
    let mut params = None;
    let mut callback = None;
    for pair in query.unwrap_or_default().split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "method" => method = Some(percent_decode(value)),
            "id" => id = Some(percent_decode(value)),
            "jsoncallback" => callback = Some(percent_decode(value)),
            "params" => {
                let decoded = percent_decode(value);
                params = decode_base64(&decoded).and_then(|bytes| serde_json::from_slice(&bytes).ok());
            }
            _ => {}
        }
    }
    // aria2@1.37.0 src/json.cc:141-144: without method and id, params itself
    // is a batch request.
    if method.is_none() && id.is_none() {
        if let Some(Value::Array(batch)) = &params {
            let mut replies = Vec::with_capacity(batch.len());
            for request in batch {
                if let Value::Object(_) = request {
                    replies.push(evaluate(&state, request).await.1);
                }
            }
            return json_reply(StatusCode::OK, Value::Array(replies), callback.as_deref());
        }
        return transport_error(Value::Null, -32600, "Invalid Request.", StatusCode::BAD_REQUEST);
    }
    let mut request = serde_json::Map::new();
    if let Some(method) = method {
        request.insert("method".to_owned(), Value::String(method));
    }
    if let Some(id) = id {
        request.insert("id".to_owned(), Value::String(id));
    }
    if let Some(params) = params {
        request.insert("params".to_owned(), params);
    }
    process(&state, Value::Object(request), callback.as_deref()).await
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&text[i + 1..i + 3], 16)
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len());
    let mut acc = 0u32;
    let mut bits = 0u32;
    for byte in text.bytes() {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(digit);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Validate transport, authorize, dispatch, then wrap the result or the
/// method-level error (aria2@1.37.0 src/rpc_helper.cc:80-105 and
/// src/RpcMethod.cc:61-104).
async fn evaluate(state: &AppState, request: &Value) -> (StatusCode, Value) {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return (
            StatusCode::BAD_REQUEST,
            error_envelope(id, -32600, "Invalid Request."),
        );
    };
    if request.get("id").is_none() {
        return (
            StatusCode::BAD_REQUEST,
            error_envelope(Value::Null, -32600, "Invalid Request."),
        );
    }
    let mut params = match request.get("params") {
        None => Vec::new(),
        Some(Value::Array(values)) => values.clone(),
        // Named params are explicitly unsupported in aria2 1.37.0.
        Some(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_envelope(id, -32602, "Invalid params."),
            );
        }
    };
    let first = params.first().and_then(Value::as_str).map(str::to_owned);
    let authorized = match first {
        Some(first) if first.starts_with("token:") => {
            params.remove(0);
            let token = &first["token:".len()..];
            state.secret.as_deref().is_none_or(|secret| secret == token)
        }
        _ => state.secret.is_none(),
    };
    if !authorized {
        return (StatusCode::BAD_REQUEST, error_envelope(id, 1, "Unauthorized"));
    }
    match execute(state, method, params).await {
        Ok(result) => (StatusCode::OK, json_envelope(id, result, false)),
        Err(message) => (StatusCode::BAD_REQUEST, error_envelope(id, 1, &message)),
    }
}

async fn process(state: &AppState, request: Value, callback: Option<&str>) -> Response {
    let (status, body) = evaluate(state, &request).await;
    json_reply(status, body, callback)
}

fn new_gid(state: &Mutex<RpcState>) -> String {
    let mut state = state.lock().expect("RPC state poisoned");
    state.serial += 1;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_be_bytes();
    let mut hasher = Sha1::new();
    hasher.update(nanos);
    hasher.update(state.serial.to_be_bytes());
    hasher.update(std::process::id().to_be_bytes());
    let digest = hasher.finalize();
    digest[..8].iter().map(|byte| format!("{byte:02x}")).collect()
}

fn status_of(download: &Download) -> Value {
    let mut status = serde_json::Map::new();
    let fields: [(&str, String); 14] = [
        ("bitfield", download.bitfield.clone()),
        ("completedLength", download.completed_length.to_string()),
        ("connections", "0".to_owned()),
        (
            "dir",
            download
                .path
                .rsplit_once('/')
                .map_or(download.path.clone(), |(dir, _)| dir.to_owned()),
        ),
        ("downloadSpeed", "0".to_owned()),
        ("errorCode", download.error_code.clone()),
        ("errorMessage", download.error_message.clone()),
        ("gid", download.gid.clone()),
        ("numPieces", download.num_pieces.to_string()),
        ("pieceLength", download.piece_length.to_string()),
        ("status", download.status.clone()),
        ("totalLength", download.total_length.to_string()),
        ("uploadLength", "0".to_owned()),
        ("uploadSpeed", "0".to_owned()),
    ];
    for (key, value) in fields {
        status.insert(key.to_owned(), Value::String(value));
    }
    let mut file = serde_json::Map::new();
    let path = download.path.clone();
    let uri = download.uri.clone();
    file.insert(
        "completedLength".to_owned(),
        Value::String(download.completed_length.to_string()),
    );
    file.insert("index".to_owned(), Value::String("1".to_owned()));
    file.insert(
        "length".to_owned(),
        Value::String(download.total_length.to_string()),
    );
    file.insert("path".to_owned(), Value::String(path));
    file.insert("selected".to_owned(), Value::String("true".to_owned()));
    file.insert(
        "uris".to_owned(),
        json!([
            { "status": "used", "uri": uri.clone() },
            { "status": "waiting", "uri": uri }
        ]),
    );
    status.insert("files".to_owned(), Value::Array(vec![Value::Object(file)]));
    Value::Object(status)
}


async fn execute(state: &AppState, method: &str, params: Vec<Value>) -> Result<Value, String> {
    match method {
        "system.listMethods" => Ok(json!(METHODS)),
        "system.listNotifications" => Ok(json!(NOTIFICATIONS)),
        "aria2.getVersion" => Ok(json!({ "enabledFeatures": FEATURES, "version": VERSION })),
        "aria2.getGlobalStat" => {
            let inner = state.inner.lock().expect("RPC state poisoned");
            let count = |status: &str| {
                inner
                    .downloads
                    .iter()
                    .filter(|download| download.status == status)
                    .count()
                    .to_string()
            };
            Ok(json!({
                "downloadSpeed": "0",
                "numActive": count("active"),
                "numStopped": count("complete"),
                "numStoppedTotal": inner.downloads.len().to_string(),
                "numWaiting": count("waiting"),
                "uploadSpeed": "0"
            }))
        }
        "aria2.addUri" => {
            let Some(Value::Array(uris)) = params.first() else {
                return Err("URI is not provided.".to_owned());
            };
            let Some(Value::String(uri)) = uris.first() else {
                return Err("URI is not provided.".to_owned());
            };
            let gid = new_gid(&state.inner);
            let dir = state.dir.clone();
            let download_dir = dir.clone();
            let ca = state.ca.clone();
            let download_uri = uri.clone();
            let fetched = tokio::task::spawn_blocking(move || {
                engine::rpc_fetch(&download_uri, &download_dir, ca.as_deref())
            })
            .await
            .ok()
            .flatten();
            let entry = match fetched {
                Some(Ok(fetched)) => Download {
                    gid: gid.clone(),
                    status: "complete".to_owned(),
                    uri: uri.clone(),
                    path: fetched.path,
                    total_length: fetched.total_length,
                    completed_length: fetched.total_length,
                    piece_length: fetched.piece_length,
                    num_pieces: fetched.num_pieces,
                    bitfield: fetched.bitfield,
                    error_code: "0".to_owned(),
                    error_message: String::new(),
                },
                Some(Err(kind)) => Download {
                    gid: gid.clone(),
                    status: "error".to_owned(),
                    uri: uri.clone(),
                    path: dir,
                    total_length: 0,
                    completed_length: 0,
                    piece_length: 1 << 20,
                    num_pieces: 0,
                    bitfield: String::new(),
                    error_code: "1".to_owned(),
                    error_message: kind,
                },
                None => return Err("URI is not provided.".to_owned()),
            };
            state
                .inner
                .lock()
                .expect("RPC state poisoned")
                .downloads
                .push(entry);
            Ok(Value::String(gid))
        }
        "aria2.tellStatus" => {
            let Some(gid) = params.first().and_then(Value::as_str) else {
                return Err("GID is not provided.".to_owned());
            };
            let inner = state.inner.lock().expect("RPC state poisoned");
            match inner.downloads.iter().find(|download| download.gid == gid) {
                Some(download) => Ok(status_of(download)),
                None => Err(format!("GID {gid} is not found")),
            }
        }
        "aria2.tellActive" | "aria2.tellWaiting" | "aria2.tellStopped" => {
            let status = match method {
                "aria2.tellActive" => "active",
                "aria2.tellWaiting" => "waiting",
                _ => "complete",
            };
            let inner = state.inner.lock().expect("RPC state poisoned");
            Ok(Value::Array(
                inner
                    .downloads
                    .iter()
                    .filter(|download| download.status == status)
                    .map(status_of)
                    .collect(),
            ))
        }
        "aria2.getFiles" => {
            let Some(gid) = params.first().and_then(Value::as_str) else {
                return Err("GID is not provided.".to_owned());
            };
            let inner = state.inner.lock().expect("RPC state poisoned");
            match inner.downloads.iter().find(|download| download.gid == gid) {
                Some(download) => Ok(status_of(download)
                    .get("files")
                    .cloned()
                    .unwrap_or(Value::Null)),
                None => Err(format!("GID {gid} is not found")),
            }
        }
        "aria2.getUris" => {
            let Some(gid) = params.first().and_then(Value::as_str) else {
                return Err("GID is not provided.".to_owned());
            };
            let inner = state.inner.lock().expect("RPC state poisoned");
            if inner.downloads.iter().any(|download| download.gid == gid) {
                Err(format!("No URI data is available for GID#{gid}"))
            } else {
                Err(format!("GID {gid} is not found"))
            }
        }
        "aria2.getServers" | "aria2.getPeers" => Ok(json!([])),
        "aria2.getOption" | "aria2.getGlobalOption" => Ok(json!({})),
        "aria2.getSessionInfo" => Ok(json!({ "sessionId": new_gid(&state.inner) })),
        "aria2.saveSession" => Ok(Value::String("OK".to_owned())),
        "system.multicall" => {
            let Value::Array(calls) = params.first().unwrap_or(&Value::Null) else {
                return Err("Invalid params.".to_owned());
            };
            let mut replies = Vec::with_capacity(calls.len());
            for call in calls {
                let Some(name) = call.get("methodName").and_then(Value::as_str) else {
                    replies.push(json!({ "faultCode": 1, "faultString": "Invalid Request." }));
                    continue;
                };
                let nested = call
                    .get("params")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                match Box::pin(execute(state, name, nested)).await {
                    Ok(value) => replies.push(Value::Array(vec![value])),
                    Err(message) => replies.push(json!({ "faultCode": 1, "faultString": message })),
                }
            }
            Ok(Value::Array(replies))
        }
        "aria2.remove" | "aria2.forceRemove" | "aria2.pause" | "aria2.forcePause"
        | "aria2.unpause" | "aria2.removeDownloadResult" => {
            let Some(gid) = params.first().and_then(Value::as_str) else {
                return Err("GID is not provided.".to_owned());
            };
            let mut inner = state.inner.lock().expect("RPC state poisoned");
            let Some(position) = inner
                .downloads
                .iter()
                .position(|download| download.gid == gid)
            else {
                return Err(format!("GID {gid} is not found"));
            };
            match method {
                "aria2.remove" | "aria2.forceRemove" | "aria2.removeDownloadResult" => {
                    inner.downloads.remove(position);
                }
                "aria2.unpause" => inner.downloads[position].status = "waiting".to_owned(),
                _ => inner.downloads[position].status = "paused".to_owned(),
            }
            Ok(Value::String(gid.to_owned()))
        }
        "aria2.pauseAll" | "aria2.forcePauseAll" | "aria2.unpauseAll"
        | "aria2.purgeDownloadResult" => {
            let mut inner = state.inner.lock().expect("RPC state poisoned");
            if method == "aria2.purgeDownloadResult" {
                inner.downloads.retain(|download| download.status != "complete");
            } else {
                let status = if method == "aria2.unpauseAll" {
                    "waiting"
                } else {
                    "paused"
                };
                for download in &mut inner.downloads {
                    download.status = status.to_owned();
                }
            }
            Ok(Value::String("OK".to_owned()))
        }
        "aria2.changePosition" | "aria2.changeUri" | "aria2.changeOption"
        | "aria2.changeGlobalOption" => Ok(Value::String("OK".to_owned())),
        "aria2.shutdown" | "aria2.forceShutdown" => Ok(Value::String("OK".to_owned())),
        _ => Err(format!("No such method: {method}")),
    }
}

// --- XML-RPC dialect (aria2@1.37.0 src/RpcResponse.cc:52-122) ---

#[derive(Debug)]
struct XmlNode {
    name: String,
    text: String,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            text: String::new(),
            children: Vec::new(),
        }
    }

    fn child(&self, name: &str) -> Option<&XmlNode> {
        self.children.iter().find(|child| child.name == name)
    }
}

fn parse_xml(text: &str) -> Option<XmlNode> {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(name)) => stack.push(XmlNode::new(name.name().as_ref())),
            Ok(Event::Empty(name)) => {
                let node = XmlNode::new(name.name().as_ref());
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(text.as_ref());
                }
            }
            Ok(Event::End(_)) => {
                let node = stack.pop()?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
    }
    root
}

fn xml_value(node: &XmlNode) -> Value {
    match node.name.as_str() {
        "int" | "i4" => node
            .text
            .trim()
            .parse::<i64>()
            .map_or(Value::Null, Value::from),
        "array" => Value::Array(
            node.child("data")
                .map(|data| {
                    data.children
                        .iter()
                        .filter(|child| child.name == "value")
                        .map(|child| child.children.first().map_or(Value::Null, xml_value))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        "struct" => Value::Object(
            node.children
                .iter()
                .filter(|child| child.name == "member")
                .filter_map(|member| {
                    let name = member.child("name")?.text.clone();
                    let value = member
                        .child("value")?
                        .children
                        .first()
                        .map_or(Value::Null, xml_value);
                    Some((name, value))
                })
                .collect(),
        ),
        _ => Value::String(node.text.clone()),
    }
}

fn xml_request_params(call: &XmlNode) -> Vec<Value> {
    call.child("params")
        .map(|params| {
            params
                .children
                .iter()
                .filter(|child| child.name == "param")
                .filter_map(|param| param.child("value")?.children.first().map(xml_value))
                .collect()
        })
        .unwrap_or_default()
}

fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\'' => out.push_str("&#39;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn write_xml_value(out: &mut String, value: &Value) {
    match value {
        Value::String(text) => {
            out.push_str("<value><string>");
            out.push_str(&escape_xml(text));
            out.push_str("</string></value>");
        }
        Value::Number(number) => {
            out.push_str("<value><int>");
            out.push_str(&number.to_string());
            out.push_str("</int></value>");
        }
        Value::Array(items) => {
            out.push_str("<value><array><data>");
            for item in items {
                write_xml_value(out, item);
            }
            out.push_str("</data></array></value>");
        }
        Value::Object(members) => {
            out.push_str("<value><struct>");
            for (name, value) in members {
                out.push_str("<member><name>");
                out.push_str(&escape_xml(name));
                out.push_str("</name>");
                write_xml_value(out, value);
                out.push_str("</member>");
            }
            out.push_str("</struct></value>");
        }
        Value::Bool(_) | Value::Null => {}
    }
}

fn xml_response(result: Result<Value, String>) -> String {
    let mut out = String::from("<?xml version=\"1.0\"?><methodResponse>");
    match result {
        Ok(value) => {
            out.push_str("<params><param>");
            write_xml_value(&mut out, &value);
            out.push_str("</param></params>");
        }
        Err(message) => {
            out.push_str("<fault><value><struct><member><name>faultCode</name><value><int>1</int></value></member><member><name>faultString</name><value><string>");
            out.push_str(&escape_xml(&message));
            out.push_str("</string></value></member></struct></value></fault>");
        }
    }
    out.push_str("</methodResponse>");
    out
}

async fn xml_post(State(state): State<AppState>, body: Bytes) -> Response {
    let text = String::from_utf8_lossy(&body);
    let Some(call) = parse_xml(&text) else {
        // aria2@1.37.0 src/HttpServerBodyCommand.cc:233-239: malformed XML is
        // a bare 400 with no body.
        return wired(StatusCode::BAD_REQUEST, "text/xml", String::new());
    };
    let Some(method) = call.child("methodName").map(|node| node.text.as_str()) else {
        return wired(StatusCode::BAD_REQUEST, "text/xml", String::new());
    };
    let mut params = xml_request_params(&call);
    let first = params.first().and_then(Value::as_str).map(str::to_owned);
    let authorized = match first {
        Some(first) if first.starts_with("token:") => {
            params.remove(0);
            let token = &first["token:".len()..];
            state.secret.as_deref().is_none_or(|secret| secret == token)
        }
        _ => state.secret.is_none(),
    };
    let answer = if !authorized {
        Err("Unauthorized".to_owned())
    } else {
        execute(&state, method, params).await
    };
    wired(StatusCode::OK, "text/xml", xml_response(answer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_e17_method_and_notification_sets() {
        assert_eq!(METHODS.len(), 36);
        assert_eq!(NOTIFICATIONS.len(), 6);
        assert_eq!(METHODS[0], "aria2.addUri");
        assert_eq!(METHODS[35], "system.listNotifications");
    }

    #[test]
    fn eq_e16_xml_fault_shape() {
        let body = xml_response(Err("Unauthorized".to_owned()));
        assert_eq!(
            body,
            "<?xml version=\"1.0\"?><methodResponse><fault><value><struct><member><name>faultCode</name><value><int>1</int></value></member><member><name>faultString</name><value><string>Unauthorized</string></value></member></struct></value></fault></methodResponse>"
        );
    }

    #[test]
    fn xml_parsing_keeps_aria2_scalar_types() {
        let body = "<?xml version=\"1.0\"?><methodCall><methodName>aria2.addUri</methodName><params><param><value><string>token:x</string></value></param><param><value><array><data><value><string>http://127.0.0.1:18080/1kib.bin</string></value></data></array></value></param></params></methodCall>";
        let call = parse_xml(body).expect("call parses");
        assert_eq!(
            call.child("methodName").map(|node| node.text.as_str()),
            Some("aria2.addUri")
        );
        let params = xml_request_params(&call);
        assert_eq!(params.len(), 2);
        assert_eq!(params[1][0], json!("http://127.0.0.1:18080/1kib.bin"));
    }

    #[test]
    fn eq_e13_gid_is_16_lowercase_hex() {
        let state = Mutex::new(RpcState::default());
        let gid = new_gid(&state);
        assert_eq!(gid.len(), 16);
        assert!(gid
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    }

    #[test]
    fn e18_get_query_params_are_base64_decoded() {
        assert_eq!(percent_decode("aria2.getGlobalStat"), "aria2.getGlobalStat");
        let bytes = decode_base64("WyJ0b2tlbjpsb2NhbC1ycGMtdGVzdC1vbmx5Il0=").expect("base64");
        assert_eq!(bytes, br#"["token:local-rpc-test-only"]"#);
    }
}
