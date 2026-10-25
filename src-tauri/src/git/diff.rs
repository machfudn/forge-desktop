use super::cli;
use std::fs;
use std::path::Path;

pub fn get_file_diff(repo: &str, file_path: &str, staged: bool) -> Result<String, String> {
    if file_path.trim().is_empty() {
        return Err("Path file tidak boleh kosong".into());
    }

    if staged {
        cli::run(repo, &["diff", "--cached", "--", file_path])
    } else {
        let diff_out = cli::run(repo, &["diff", "--", file_path])?;
        if !diff_out.is_empty() {
            return Ok(diff_out);
        }

        // Jika diff unstaged kosong, cek jika file baru/untracked
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_empty_file_path() {
        assert!(get_file_diff("/tmp", "  ", false).is_err());
    }
}
