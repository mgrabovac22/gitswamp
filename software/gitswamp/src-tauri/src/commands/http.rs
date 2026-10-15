use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 300_000;
const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HttpHeaderPair {
    pub name: String,
    pub value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpSendPayload {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HttpHeaderPair>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub follow_redirects: Option<bool>,
    #[serde(default)]
    pub max_redirects: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponsePayload {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<HttpHeaderPair>,
    pub body: String,
    pub body_base64: Option<String>,
    pub is_binary: bool,
    pub size_bytes: usize,
    pub duration_ms: u64,
    pub final_url: String,
}

fn is_textual_content_type(content_type: &str) -> bool {
    let value = content_type.to_ascii_lowercase();
    if value.is_empty() {
        return true;
    }

    value.starts_with("text/")
        || value.contains("json")
        || value.contains("xml")
        || value.contains("javascript")
        || value.contains("ecmascript")
        || value.contains("x-www-form-urlencoded")
        || value.contains("graphql")
        || value.contains("yaml")
        || value.contains("csv")
        || value.contains("html")
}

fn collect_headers(response: &ureq::Response) -> Vec<HttpHeaderPair> {
    let mut seen = Vec::new();
    let mut headers = Vec::new();

    for name in response.headers_names() {
        if seen.contains(&name) {
            continue;
        }
        seen.push(name.clone());

        for value in response.all(&name) {
            headers.push(HttpHeaderPair {
                name: name.clone(),
                value: value.to_string(),
            });
        }
    }

    headers
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

fn execute(payload: HttpSendPayload) -> Result<HttpResponsePayload, String> {
    let method = payload.method.trim().to_ascii_uppercase();
    let url = payload.url.trim().to_string();

    if method.is_empty() {
        return Err("HTTP method is required.".to_string());
    }
    if url.is_empty() {
        return Err("Request URL is required.".to_string());
    }

    let timeout = Duration::from_millis(
        payload
            .timeout_ms
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(1_000, MAX_TIMEOUT_MS),
    );
    let redirects = if payload.follow_redirects.unwrap_or(true) {
        payload.max_redirects.unwrap_or(10)
    } else {
        0
    };

    let agent = ureq::AgentBuilder::new()
        .timeout(timeout)
        .redirects(redirects)
        .user_agent("GitSwamp/0.3 HTTP Client")
        .build();

    let body = payload.body.clone().unwrap_or_default();
    let has_body = !body.is_empty() || payload.body.is_some();
    let should_send_body = has_body && method != "GET" && method != "HEAD";

    let mut request = agent.request(&method, &url);
    for header in &payload.headers {
        let name = header.name.trim();
        if name.is_empty() {
            continue;
        }
        request = request.set(name, &header.value);
    }

    let started = Instant::now();
    let result = if should_send_body {
        request.send_string(&body)
    } else {
        request.call()
    };
    let duration_ms = started.elapsed().as_millis() as u64;

    let response = match result {
        Ok(response) => response,
        Err(ureq::Error::Status(_code, response)) => response,
        Err(ureq::Error::Transport(transport)) => {
            return Err(format!("Request failed: {}", transport));
        }
    };

    let status = response.status();
    let status_text = response.status_text().to_string();
    let final_url = response.get_url().to_string();
    let content_type = response.header("content-type").unwrap_or("").to_string();
    let headers = collect_headers(&response);

    let mut reader = response.into_reader().take(MAX_BODY_BYTES);
    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let size_bytes = buffer.len();
    let is_binary = !is_textual_content_type(&content_type);

    let (body_text, body_base64) = if is_binary {
        (String::new(), Some(base64_encode(&buffer)))
    } else {
        (String::from_utf8_lossy(&buffer).into_owned(), None)
    };

    Ok(HttpResponsePayload {
        status,
        status_text,
        headers,
        body: body_text,
        body_base64,
        is_binary,
        size_bytes,
        duration_ms,
        final_url,
    })
}

#[tauri::command]
pub async fn http_send_request(payload: HttpSendPayload) -> Result<HttpResponsePayload, String> {
    tauri::async_runtime::spawn_blocking(move || execute(payload))
        .await
        .map_err(|e| format!("HTTP task failed: {}", e))?
}
