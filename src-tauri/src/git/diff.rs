use super::cli;
use std::fs;
use std::path::Path;

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
