use super::cli;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

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
    if p.is_empty() || p.starts_with('-') || p.contains("..") {
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
    let p = file_path.trim();
    if p.is_empty() || p.starts_with('-') || p.contains("..") {
        return Err("Path file tidak boleh kosong atau tidak valid".into());
    }

    let mut args = vec!["diff"];
    if ignore_whitespace {
        args.push("-w");
    }
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(p);

    let diff_out = cli::run(repo, &args)?;
    if !diff_out.is_empty() || staged {
        return Ok(diff_out);
    }

    let full_path = Path::new(repo).join(p);
    if full_path.exists() && full_path.is_file() {
        if let Ok(content) = fs::read_to_string(&full_path) {
            let lines: Vec<String> = content.lines().map(|l| format!("+{l}")).collect();
            return Ok(format!(
                "--- /dev/null\n+++ b/{p}\n@@ -0,0 +1,{} @@\n{}",
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

pub fn stage_hunk(repo: &str, patch_text: &str, reverse: bool) -> Result<(), String> {
    if !Path::new(repo).join(".git").exists() {
        return Err("Folder bukan repository Git".into());
    }
    if patch_text.trim().is_empty() {
        return Err("Patch text tidak boleh kosong".into());
    }
    let mut args = vec!["-C", repo, "apply", "--cached"];
    if reverse {
        args.push("--reverse");
    }
    args.push("-");

    let mut child = crate::system::command_no_window("git")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Gagal menjalankan git apply: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(patch_text.as_bytes())
            .map_err(|e| format!("Gagal menulis patch ke stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Gagal menyelesaikan git apply: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if err.is_empty() {
            "Gagal melakukan apply hunk ke index".into()
        } else {
            err
        });
    }
    Ok(())
}

fn parse_hunk_starts(header: &str) -> Option<(usize, usize)> {
    // @@ -oldStart[,oldCount] +newStart[,newCount] @@...
    let rest = header.strip_prefix("@@ -")?;
    let (old_part, after_old) = rest.split_once(" +")?;
    let (new_part, _) = after_old.split_once(" @@")?;
    let old_start = old_part
        .split_once(',')
        .map(|(s, _)| s)
        .unwrap_or(old_part)
        .trim()
        .parse::<usize>()
        .ok()?;
    let new_start = new_part
        .split_once(',')
        .map(|(s, _)| s)
        .unwrap_or(new_part)
        .trim()
        .parse::<usize>()
        .ok()?;
    Some((old_start, new_start))
}

pub fn build_selective_hunk_patch(
    hunk_patch: &str,
    selected_indices: &[usize],
    reverse: bool,
) -> Result<String, String> {
    use std::collections::HashSet;
    let selected: HashSet<usize> = selected_indices.iter().copied().collect();
    if selected.is_empty() {
        return Err("Pilih minimal satu baris perubahan".into());
    }

    let normalized = hunk_patch.replace("\r\n", "\n");
    let raw_lines: Vec<&str> = normalized.split('\n').collect();
    let mut header_lines: Vec<&str> = Vec::new();
    let mut hunk_header_idx: Option<usize> = None;

    for (i, line) in raw_lines.iter().enumerate() {
        if line.starts_with("@@ ") {
            hunk_header_idx = Some(i);
            break;
        }
        header_lines.push(line);
    }

    let h_idx = hunk_header_idx.ok_or_else(|| "Header hunk @@ tidak ditemukan".to_string())?;
    let hunk_header = raw_lines[h_idx];
    let (old_start, new_start) =
        parse_hunk_starts(hunk_header).ok_or_else(|| "Format header hunk tidak valid".to_string())?;
    let suffix = hunk_header
        .find(" @@")
        .map(|pos| &hunk_header[pos + 3..])
        .unwrap_or("");

    let mut body_slice = &raw_lines[h_idx + 1..];
    if body_slice.last() == Some(&"") {
        body_slice = &body_slice[..body_slice.len() - 1];
    }

    let mut out_body: Vec<String> = Vec::new();
    let mut old_count: usize = 0;
    let mut new_count: usize = 0;
    let mut has_changes = false;
    let mut last_kept = false;

    for (idx, line) in body_slice.iter().enumerate() {
        if line.starts_with('\\') {
            if last_kept {
                out_body.push((*line).to_string());
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix('+') {
            let is_sel = selected.contains(&idx);
            if !reverse {
                if is_sel {
                    out_body.push(format!("+{rest}"));
                    new_count += 1;
                    has_changes = true;
                    last_kept = true;
                } else {
                    last_kept = false;
                }
            } else if is_sel {
                out_body.push(format!("+{rest}"));
                new_count += 1;
                has_changes = true;
                last_kept = true;
            } else {
                out_body.push(format!(" {rest}"));
                old_count += 1;
                new_count += 1;
                last_kept = true;
            }
        } else if let Some(rest) = line.strip_prefix('-') {
            let is_sel = selected.contains(&idx);
            if !reverse {
                if is_sel {
                    out_body.push(format!("-{rest}"));
                    old_count += 1;
                    has_changes = true;
                    last_kept = true;
                } else {
                    out_body.push(format!(" {rest}"));
                    old_count += 1;
                    new_count += 1;
                    last_kept = true;
                }
            } else if is_sel {
                out_body.push(format!("-{rest}"));
                old_count += 1;
                has_changes = true;
                last_kept = true;
            } else {
                last_kept = false;
            }
        } else {
            out_body.push((*line).to_string());
            old_count += 1;
            new_count += 1;
            last_kept = true;
        }
    }

    if !has_changes {
        return Err("Tidak ada baris perubahan yang dipilih".into());
    }

    let adj_old_start = if old_count == 0 && old_start > 0 {
        old_start
    } else {
        old_start
    };
    let adj_new_start = if new_count == 0 && new_start > 0 {
        new_start
    } else {
        new_start
    };

    let mut out = String::new();
    for h in header_lines {
        out.push_str(h);
        out.push('\n');
    }
    out.push_str(&format!(
        "@@ -{adj_old_start},{old_count} +{adj_new_start},{new_count} @@{suffix}\n"
    ));
    for l in out_body {
        out.push_str(&l);
        out.push('\n');
    }
    Ok(out)
}

pub fn stage_lines(
    repo: &str,
    hunk_patch: &str,
    selected_indices: Vec<usize>,
    reverse: bool,
) -> Result<(), String> {
    let selective_patch = build_selective_hunk_patch(hunk_patch, &selected_indices, reverse)?;
    stage_hunk(repo, &selective_patch, reverse)
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

    #[test]
    fn reject_empty_stage_hunk() {
        assert!(stage_hunk("/tmp", "   ", false).is_err());
    }

    #[test]
    fn selective_stage_and_unstage_lines_on_real_repo() {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("forge-line-stage-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        let run_git = |args: &[&str]| {
            let out = Command::new("git").args(args).current_dir(&root).output().unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };

        run_git(&["init"]);
        run_git(&["config", "user.name", "Tester"]);
        run_git(&["config", "user.email", "tester@example.com"]);
        std::fs::write(root.join("code.txt"), "line1\nline2\nline3\n").unwrap();
        run_git(&["add", "code.txt"]);
        run_git(&["commit", "-m", "init"]);

        std::fs::write(root.join("code.txt"), "line1\nline2_mod\nline3\nline4_add\n").unwrap();
        let diff = get_file_diff(root.to_str().unwrap(), "code.txt", false, false).unwrap();
        let hunk_start = diff.find("@@ ").unwrap();
        let patch = format!("--- a/code.txt\n+++ b/code.txt\n{}", &diff[hunk_start..]);
        // Body lines: 0=' line1', 1='-line2', 2='+line2_mod', 3=' line3', 4='+line4_add'
        // Stage only line4_add (index 4)
        stage_lines(root.to_str().unwrap(), &patch, vec![4], false).unwrap();
        let staged_diff = get_file_diff(root.to_str().unwrap(), "code.txt", true, false).unwrap();
        assert!(staged_diff.contains("+line4_add"));
        assert!(!staged_diff.contains("+line2_mod"));

        // Now unstage line4_add
        let staged_hunk = staged_diff.find("@@ ").unwrap();
        let staged_patch = format!("--- a/code.txt\n+++ b/code.txt\n{}", &staged_diff[staged_hunk..]);
        let body_lines: Vec<&str> = staged_diff[staged_hunk..].lines().skip(1).collect();
        let add_idx = body_lines.iter().position(|l| *l == "+line4_add").unwrap();
        stage_lines(root.to_str().unwrap(), &staged_patch, vec![add_idx], true).unwrap();
        let after_unstage = get_file_diff(root.to_str().unwrap(), "code.txt", true, false).unwrap();
        assert!(after_unstage.trim().is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
