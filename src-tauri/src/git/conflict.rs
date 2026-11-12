use super::cli;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictBlock {
    pub index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub ours_label: String,
    pub theirs_label: String,
    pub ours: String,
    pub base: Option<String>,
    pub theirs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictFileData {
    pub file_path: String,
    pub base_content: Option<String>,
    pub ours_content: Option<String>,
    pub theirs_content: Option<String>,
    pub working_content: String,
    pub blocks: Vec<ConflictBlock>,
}

fn validate_repo_rel_path(repo: &str, file_path: &str) -> Result<PathBuf, String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return Err("Path file tidak valid".into());
    }
    let rel = Path::new(trimmed);
    if rel.is_absolute() {
        return Err("Path file tidak boleh absolut".into());
    }
    for comp in rel.components() {
        if matches!(comp, Component::ParentDir | Component::RootDir | Component::Prefix(_)) {
            return Err("Path file mengandung traversal tidak aman".into());
        }
    }
    Ok(Path::new(repo).join(rel))
}

pub fn parse_conflict_blocks(content: &str) -> Vec<ConflictBlock> {
    let normalized = content.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if let Some(rest_ours) = lines[i].strip_prefix("<<<<<<<") {
            let start_line = i + 1;
            let ours_label = rest_ours.trim().to_string();
            let mut ours_lines: Vec<&str> = Vec::new();
            let mut base_lines: Vec<&str> = Vec::new();
            let mut theirs_lines: Vec<&str> = Vec::new();
            let mut mode = 0u8; // 0 = ours, 1 = base, 2 = theirs
            let mut has_base = false;
            let mut j = i + 1;
            let mut closed = false;

            while j < lines.len() {
                let l = lines[j];
                if l.starts_with("|||||||") && mode == 0 {
                    has_base = true;
                    mode = 1;
                } else if l.starts_with("=======") && mode < 2 {
                    mode = 2;
                } else if let Some(rest_theirs) = l.strip_prefix(">>>>>>>") {
                    let theirs_label = rest_theirs.trim().to_string();
                    blocks.push(ConflictBlock {
                        index: blocks.len(),
                        start_line,
                        end_line: j + 1,
                        ours_label: if ours_label.is_empty() {
                            "HEAD (Ours)".into()
                        } else {
                            ours_label
                        },
                        theirs_label: if theirs_label.is_empty() {
                            "Incoming (Theirs)".into()
                        } else {
                            theirs_label
                        },
                        ours: ours_lines.join("\n"),
                        base: if has_base {
                            Some(base_lines.join("\n"))
                        } else {
                            None
                        },
                        theirs: theirs_lines.join("\n"),
                    });
                    i = j + 1;
                    closed = true;
                    break;
                } else {
                    match mode {
                        0 => ours_lines.push(l),
                        1 => base_lines.push(l),
                        _ => theirs_lines.push(l),
                    }
                }
                j += 1;
            }
            if !closed {
                break;
            }
        } else {
            i += 1;
        }
    }
    blocks
}

pub fn get_conflict_file(repo: &str, file_path: &str) -> Result<ConflictFileData, String> {
    let full_path = validate_repo_rel_path(repo, file_path)?;
    let working_content = fs::read_to_string(&full_path)
        .map_err(|e| format!("Gagal membaca file konflik: {e}"))?;

    let rel = file_path.trim().replace('\\', "/");
    let base_content = cli::run(repo, &["show", &format!(":1:{rel}")]).ok();
    let ours_content = cli::run(repo, &["show", &format!(":2:{rel}")]).ok();
    let theirs_content = cli::run(repo, &["show", &format!(":3:{rel}")]).ok();
    let blocks = parse_conflict_blocks(&working_content);

    Ok(ConflictFileData {
        file_path: rel,
        base_content,
        ours_content,
        theirs_content,
        working_content,
        blocks,
    })
}

pub fn resolve_conflict_file(
    repo: &str,
    file_path: &str,
    resolved_content: &str,
    stage_after: bool,
) -> Result<(), String> {
    let full_path = validate_repo_rel_path(repo, file_path)?;
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Gagal membuat direktori: {e}"))?;
    }
    fs::write(&full_path, resolved_content)
        .map_err(|e| format!("Gagal menyimpan hasil resolusi konflik: {e}"))?;
    if stage_after {
        let rel = file_path.trim().replace('\\', "/");
        cli::run(repo, &["add", "--", &rel])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_way_and_diff3_conflicts() {
        let sample = "\
fn header() {}
<<<<<<< HEAD
let x = 1;
=======
let x = 2;
>>>>>>> feat/two
fn middle() {}
<<<<<<< ours
let y = 10;
||||||| base
let y = 0;
=======
let y = 20;
>>>>>>> theirs
";
        let blocks = parse_conflict_blocks(sample);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].ours_label, "HEAD");
        assert_eq!(blocks[0].theirs_label, "feat/two");
        assert_eq!(blocks[0].ours, "let x = 1;");
        assert_eq!(blocks[0].theirs, "let x = 2;");
        assert!(blocks[0].base.is_none());

        assert_eq!(blocks[1].base.as_deref(), Some("let y = 0;"));
        assert_eq!(blocks[1].ours, "let y = 10;");
        assert_eq!(blocks[1].theirs, "let y = 20;");
    }

    #[test]
    fn reject_traversal_in_conflict_path() {
        assert!(validate_repo_rel_path("/tmp/repo", "../etc/passwd").is_err());
        assert!(validate_repo_rel_path("/tmp/repo", "/abs/path").is_err());
    }
}
