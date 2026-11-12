use std::path::Path;
use std::process::Command;

pub fn command_no_window<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub fn open_in_file_manager(path: &str) -> Result<(), String> {
    let p = Path::new(path.trim());
    if !p.exists() {
        return Err("Path tidak ditemukan".into());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(p)
            .spawn()
            .map_err(|e| format!("Gagal membuka file manager: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(p)
            .spawn()
            .map_err(|e| format!("Gagal membuka file manager: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(p)
            .spawn()
            .map_err(|e| format!("Gagal membuka file manager: {e}"))?;
    }

    Ok(())
}

pub fn open_in_terminal(path: &str) -> Result<(), String> {
    let p = Path::new(path.trim());
    if !p.exists() {
        return Err("Path tidak ditemukan".into());
    }

    #[cfg(target_os = "windows")]
    {
        let opened = Command::new("wt.exe")
            .arg("-d")
            .arg(p)
            .spawn()
            .is_ok();
        if !opened {
            Command::new("powershell.exe")
                .arg("-NoExit")
                .current_dir(p)
                .spawn()
                .map_err(|e| format!("Gagal membuka terminal: {e}"))?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal", path])
            .spawn()
            .map_err(|e| format!("Gagal membuka terminal: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        let terminals = [
            "x-terminal-emulator",
            "gnome-terminal",
            "ptyxis",
            "alacritty",
            "kitty",
            "konsole",
            "xfce4-terminal",
            "xterm",
        ];
        let mut started = false;
        for term in terminals {
            if Command::new(term).current_dir(p).spawn().is_ok() {
                started = true;
                break;
            }
        }
        if !started {
            return Err("Terminal emulator tidak ditemukan pada sistem".into());
        }
    }

    Ok(())
}

fn is_system_default_editor(editor_name: Option<&str>) -> bool {
    match editor_name.map(str::trim) {
        None | Some("") | Some("system") | Some("default") => true,
        _ => false,
    }
}

pub fn validate_editor_command(raw: &str) -> Result<&str, String> {
    let editor = raw.trim();
    if editor.is_empty()
        || editor.starts_with('-')
        || editor.chars().any(|c| {
            c.is_control()
                || matches!(c, ';' | '&' | '|' | '`' | '$' | '>' | '<' | '"' | '\'')
        })
    {
        return Err("Nama atau path editor tidak valid".into());
    }
    Ok(editor)
}

#[cfg(target_os = "windows")]
fn is_windows_executable_or_script(p: &Path) -> bool {
    let Some(ext) = p.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "bat"
            | "cmd"
            | "ps1"
            | "vbs"
            | "vbe"
            | "js"
            | "jse"
            | "wsf"
            | "wsh"
            | "exe"
            | "msi"
            | "com"
            | "scr"
            | "lnk"
            | "hta"
    )
}

#[cfg(target_os = "windows")]
fn parse_registry_command_exe(command_value: &str) -> Option<String> {
    let trimmed = command_value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        let exe = rest[..end].trim();
        return (!exe.is_empty()).then(|| exe.to_string());
    }
    trimmed
        .split_whitespace()
        .next()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

#[cfg(target_os = "windows")]
fn resolve_windows_txt_default_exe() -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let user_choice = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.txt\UserChoice",
            "/v",
            "ProgId",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&user_choice.stdout);
    let prog_id = stdout
        .lines()
        .find_map(|line| {
            let idx = line.find("REG_SZ")?;
            let val = line[idx + "REG_SZ".len()..].trim();
            (!val.is_empty()).then(|| val.to_string())
        })?;

    let cmd_key = format!(r"HKCR\{prog_id}\shell\open\command");
    let open_cmd = Command::new("reg")
        .args(["query", &cmd_key, "/ve"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !open_cmd.status.success() {
        return None;
    }
    let cmd_out = String::from_utf8_lossy(&open_cmd.stdout);
    let raw_cmd = cmd_out.lines().find_map(|line| {
        let idx = line.find("REG_SZ").or_else(|| line.find("REG_EXPAND_SZ"))?;
        let marker_len = if line[idx..].starts_with("REG_EXPAND_SZ") {
            "REG_EXPAND_SZ".len()
        } else {
            "REG_SZ".len()
        };
        let val = line[idx + marker_len..].trim();
        (!val.is_empty()).then(|| val.to_string())
    })?;

    parse_registry_command_exe(&raw_cmd).filter(|exe| Path::new(exe).is_file())
}

#[cfg(target_os = "windows")]
fn is_folder_capable_editor_exe(exe_path: &str) -> bool {
    let name = Path::new(exe_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "code.exe"
            | "code - insiders.exe"
            | "cursor.exe"
            | "windsurf.exe"
            | "zed.exe"
            | "subl.exe"
            | "sublime_text.exe"
            | "notepad++.exe"
            | "codium.exe"
            | "devenv.exe"
            | "idea64.exe"
    )
}

#[cfg(target_os = "windows")]
fn windows_known_editor_paths(editor: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let local_app = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from);
    let prog_files = std::env::var_os("ProgramFiles").map(std::path::PathBuf::from);
    let prog_files_x86 = std::env::var_os("ProgramFiles(x86)").map(std::path::PathBuf::from);

    match editor.to_ascii_lowercase().as_str() {
        "code" => {
            if let Some(d) = &local_app {
                out.push(d.join(r"Programs\Microsoft VS Code\Code.exe"));
            }
            if let Some(d) = &prog_files {
                out.push(d.join(r"Microsoft VS Code\Code.exe"));
            }
        }
        "cursor" => {
            if let Some(d) = &local_app {
                out.push(d.join(r"Programs\cursor\Cursor.exe"));
            }
            if let Some(d) = &prog_files {
                out.push(d.join(r"cursor\Cursor.exe"));
            }
        }
        "windsurf" => {
            if let Some(d) = &local_app {
                out.push(d.join(r"Programs\Windsurf\Windsurf.exe"));
            }
        }
        "zed" => {
            if let Some(d) = &local_app {
                out.push(d.join(r"Programs\Zed\Zed.exe"));
            }
        }
        "subl" => {
            if let Some(d) = &prog_files {
                out.push(d.join(r"Sublime Text\subl.exe"));
                out.push(d.join(r"Sublime Text 3\subl.exe"));
            }
        }
        "notepad++" => {
            if let Some(d) = &prog_files {
                out.push(d.join(r"Notepad++\notepad++.exe"));
            }
            if let Some(d) = &prog_files_x86 {
                out.push(d.join(r"Notepad++\notepad++.exe"));
            }
        }
        _ => {}
    }
    out
}

fn open_with_system_default(p: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if p.is_file() && is_windows_executable_or_script(p) {
            if let Some(txt_exe) = resolve_windows_txt_default_exe() {
                if Command::new(&txt_exe).arg(p).spawn().is_ok() {
                    return Ok(());
                }
            }
            Command::new("notepad.exe")
                .arg(p)
                .spawn()
                .map_err(|e| format!("Gagal membuka file dengan editor teks default sistem: {e}"))?;
            return Ok(());
        }

        if p.is_dir() {
            if let Some(txt_exe) = resolve_windows_txt_default_exe() {
                if is_folder_capable_editor_exe(&txt_exe)
                    && Command::new(&txt_exe).arg(p).spawn().is_ok()
                {
                    return Ok(());
                }
            }
        }

        if tauri_plugin_opener::open_path(p, None::<&str>).is_ok() {
            return Ok(());
        }
        if p.is_file() {
            Command::new("notepad.exe")
                .arg(p)
                .spawn()
                .map_err(|e| format!("Gagal membuka file dengan aplikasi default sistem: {e}"))?;
            return Ok(());
        }
        return Err("Gagal membuka path dengan aplikasi default sistem".into());
    }

    #[cfg(target_os = "macos")]
    {
        if tauri_plugin_opener::open_path(p, None::<&str>).is_ok() {
            return Ok(());
        }
        if p.is_file() && Command::new("open").arg("-t").arg(p).spawn().is_ok() {
            return Ok(());
        }
        Err("Gagal membuka dengan aplikasi default sistem".into())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        tauri_plugin_opener::open_path(p, None::<&str>)
            .map_err(|e| format!("Gagal membuka dengan aplikasi default sistem: {e}"))
    }
}

pub fn open_in_editor(path: &str, editor_name: Option<&str>) -> Result<(), String> {
    let p = Path::new(path.trim());
    if !p.exists() {
        return Err("Path tidak ditemukan".into());
    }

    if is_system_default_editor(editor_name) {
        return open_with_system_default(p);
    }

    let editor = validate_editor_command(editor_name.unwrap_or(""))?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        if p.is_dir() && matches!(editor.to_ascii_lowercase().as_str(), "notepad" | "notepad.exe") {
            return Err(
                "Notepad hanya mendukung membuka file, bukan folder repositori. Pilih file terlebih dahulu atau ubah editor di Pengaturan."
                    .into(),
            );
        }

        if editor.contains('/') || editor.contains('\\') {
            if Command::new(editor).arg(p).spawn().is_ok() {
                return Ok(());
            }
            return Err(format!(
                "Editor '{editor}' tidak ditemukan atau gagal dijalankan. Periksa pengaturan Editor Eksternal."
            ));
        }

        if Command::new(editor).arg(p).spawn().is_ok() {
            return Ok(());
        }
        if !editor.to_ascii_lowercase().ends_with(".cmd")
            && Command::new(format!("{editor}.cmd"))
                .arg(p)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .is_ok()
        {
            return Ok(());
        }
        if !editor.to_ascii_lowercase().ends_with(".exe")
            && Command::new(format!("{editor}.exe")).arg(p).spawn().is_ok()
        {
            return Ok(());
        }
        for candidate in windows_known_editor_paths(editor) {
            if candidate.is_file() && Command::new(&candidate).arg(p).spawn().is_ok() {
                return Ok(());
            }
        }
        return Err(format!(
            "Editor '{editor}' tidak ditemukan atau gagal dijalankan. Periksa pengaturan Editor Eksternal."
        ));
    }

    #[cfg(target_os = "macos")]
    {
        let normalized = editor.trim_end_matches(['/', '\\']);
        if normalized.ends_with(".app") {
            if Command::new("open").args(["-a", normalized]).arg(p).spawn().is_ok() {
                return Ok(());
            }
        } else {
            if Command::new(editor).arg(p).spawn().is_ok() {
                return Ok(());
            }
            let mac_app = match editor.to_ascii_lowercase().as_str() {
                "code" => Some("Visual Studio Code"),
                "cursor" => Some("Cursor"),
                "windsurf" => Some("Windsurf"),
                "zed" => Some("Zed"),
                "subl" => Some("Sublime Text"),
                _ => None,
            };
            if let Some(app_name) = mac_app {
                if Command::new("open").args(["-a", app_name]).arg(p).spawn().is_ok() {
                    return Ok(());
                }
            }
        }
        return Err(format!(
            "Editor '{editor}' tidak ditemukan atau gagal dijalankan. Periksa pengaturan Editor Eksternal."
        ));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if Command::new(editor).arg(p).spawn().is_ok() {
            return Ok(());
        }
        let linux_fallback = match editor {
            "gedit" => Some("gnome-text-editor"),
            "zed" => Some("zeditor"),
            _ => None,
        };
        if let Some(fallback_cmd) = linux_fallback {
            if Command::new(fallback_cmd).arg(p).spawn().is_ok() {
                return Ok(());
            }
        }
        Err(format!(
            "Editor '{editor}' tidak ditemukan atau gagal dijalankan. Periksa pengaturan Editor Eksternal."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_system_default_editor_aliases() {
        assert!(is_system_default_editor(None));
        assert!(is_system_default_editor(Some("")));
        assert!(is_system_default_editor(Some("   ")));
        assert!(is_system_default_editor(Some("system")));
        assert!(is_system_default_editor(Some("default")));
        assert!(!is_system_default_editor(Some("code")));
    }

    #[test]
    fn validates_preset_and_custom_editor_paths() {
        assert_eq!(validate_editor_command("code").unwrap(), "code");
        assert_eq!(validate_editor_command("notepad++").unwrap(), "notepad++");
        assert_eq!(
            validate_editor_command(r"C:\Program Files\Notepad++\notepad++.exe").unwrap(),
            r"C:\Program Files\Notepad++\notepad++.exe"
        );
        assert_eq!(
            validate_editor_command("/Applications/Sublime Text.app").unwrap(),
            "/Applications/Sublime Text.app"
        );
        assert!(validate_editor_command("").is_err());
        assert!(validate_editor_command("-e").is_err());
        assert!(validate_editor_command("code;calc").is_err());
        assert!(validate_editor_command("code && whoami").is_err());
        assert!(validate_editor_command("code$(id)").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_registry_command_executable_paths() {
        assert_eq!(
            parse_registry_command_exe(r#""C:\Program Files\Notepad++\notepad++.exe" "%1""#).as_deref(),
            Some(r"C:\Program Files\Notepad++\notepad++.exe")
        );
        assert_eq!(
            parse_registry_command_exe(r"C:\Windows\system32\NOTEPAD.EXE %1").as_deref(),
            Some(r"C:\Windows\system32\NOTEPAD.EXE")
        );
        assert!(parse_registry_command_exe("").is_none());
    }
}
