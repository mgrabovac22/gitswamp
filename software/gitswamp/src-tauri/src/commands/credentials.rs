use std::fs;
use std::path::PathBuf;

fn credentials_path() -> PathBuf {
    let mut dir = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    dir.push(".gitswamp");
    let _ = fs::create_dir_all(&dir);
    dir.push("credentials");
    dir
}

fn dirs_next() -> Option<PathBuf> {
    // Use APPDATA on Windows, HOME/.config on others
    std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
}

// Simple XOR obfuscation with a fixed key (not true encryption, but hides plaintext)
fn obfuscate(data: &[u8]) -> Vec<u8> {
    let key = b"GitSwamp2026SecretKey!";
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect()
}

#[tauri::command]
pub fn save_token(token: String) -> Result<(), String> {
    let path = credentials_path();
    let obfuscated = obfuscate(token.as_bytes());
    let encoded = base64_encode(&obfuscated);
    fs::write(&path, encoded).map_err(|e| format!("Failed to save token: {}", e))
}

#[tauri::command]
pub fn load_token() -> Result<Option<String>, String> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(None);
    }
    let encoded = fs::read_to_string(&path).map_err(|e| format!("Failed to read token: {}", e))?;
    let encoded = encoded.trim();
    if encoded.is_empty() {
        return Ok(None);
    }
    let obfuscated = base64_decode(encoded).map_err(|e| format!("Failed to decode token: {}", e))?;
    let bytes = obfuscate(&obfuscated);
    String::from_utf8(bytes)
        .map(|s| Some(s))
        .map_err(|e| format!("Invalid token data: {}", e))
}

#[tauri::command]
pub fn delete_token() -> Result<(), String> {
    let path = credentials_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete token: {}", e))?;
    }
    Ok(())
}

// Minimal base64 encode/decode to avoid adding a dependency
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("Invalid base64 char: {}", c as char)),
        }
    }
    let bytes = input.as_bytes();
    let mut result = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = val(chunk[2])?;
        let d = val(chunk[3])?;
        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((triple & 0xFF) as u8);
        }
    }
    Ok(result)
}
