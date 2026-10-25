use super::cli;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDiffData {
    pub old_image_url: Option<String>,
    pub new_image_url: Option<String>,
    pub extension: String,
}

fn to_base64(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        result.push(CHARSET[(b0 >> 2) as usize] as char);
        result.push(CHARSET[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

pub fn get_image_diff(repo: &str, file_path: &str) -> Result<ImageDiffData, String> {
    let p = file_path.trim();
    if p.is_empty() || p.starts_with('-') {
        return Err("Path file tidak valid".into());
    }
    let ext = Path::new(p)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        _ => return Err("Bukan file format gambar yang didukung".into()),
    };

    let old_bytes = cli::run_bytes(repo, &["show", &format!("HEAD:{p}")]).ok();
    let old_image_url = old_bytes.map(|b| format!("data:{mime};base64,{}", to_base64(&b)));

    let full_new_path = Path::new(repo).join(p);
    let new_bytes = fs::read(&full_new_path).ok();
    let new_image_url = new_bytes.map(|b| format!("data:{mime};base64,{}", to_base64(&b)));

    Ok(ImageDiffData {
        old_image_url,
        new_image_url,
        extension: ext,
    })
}

pub fn get_file_diff(
    repo: &str,
    file_path: &str,
    staged: bool,
    ignore_whitespace: bool,
) -> Result<String, String> {
    if file_path.trim().is_empty() {
        return Err("Path file tidak boleh kosong".into());
    }

    let mut args = vec!["diff"];
    if ignore_whitespace {
        args.push("-w");
    }
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(file_path);

    let diff_out = cli::run(repo, &args)?;
    if !diff_out.is_empty() || staged {
        return Ok(diff_out);
    }

    let full_path = Path::new(repo).join(file_path);
    if full_path.exists() && full_path.is_file() {
        if let Ok(content) = fs::read_to_string(&full_path) {
            let lines: Vec<String> = content.lines().map(|l| format!("+{l}")).collect();
            return Ok(format!(
                "--- /dev/null\n+++ b/{file_path}\n@@ -0,0 +1,{} @@\n{}",
                lines.len(),
                lines.join("\n")
            ));
        }
    }
    Ok(String::new())
}

pub fn add_to_gitignore(repo: &str, pattern: &str) -> Result<(), String> {
    let pat = pattern.trim();
    if pat.is_empty() {
        return Err("Pattern gitignore tidak boleh kosong".into());
    }
    let gi_path = Path::new(repo).join(".gitignore");
    let mut current = if gi_path.exists() {
        fs::read_to_string(&gi_path).map_err(|e| format!("Gagal membaca .gitignore: {e}"))?
    } else {
        String::new()
    };
    if !current.ends_with('\n') && !current.is_empty() {
        current.push('\n');
    }
    current.push_str(pat);
    current.push('\n');
    fs::write(gi_path, current).map_err(|e| format!("Gagal menyimpan .gitignore: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_empty_file_path() {
        assert!(get_file_diff("/tmp", "  ", false, false).is_err());
    }

    #[test]
    fn reject_empty_gitignore_pattern() {
        assert!(add_to_gitignore("/tmp", "   ").is_err());
    }
}
