use std::path::Path;
use std::process::Command;

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
        Command::new("cmd")
            .args(["/c", "start", "wt.exe", "-d", path])
            .spawn()
            .or_else(|_| {
                Command::new("cmd")
                    .args(["/c", "start", "powershell.exe", "-NoExit", "-Command", &format!("Set-Location '{}'", path)])
                    .spawn()
            })
            .map_err(|e| format!("Gagal membuka terminal: {e}"))?;
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

pub fn open_in_editor(path: &str, editor_name: Option<&str>) -> Result<(), String> {
    let p = Path::new(path.trim());
    if !p.exists() {
        return Err("Path tidak ditemukan".into());
    }

    let editor = editor_name.unwrap_or("code").trim();
    if editor.is_empty() || editor.starts_with('-') {
        return Err("Nama editor tidak valid".into());
    }

    let candidates = [editor, "code", "cursor", "subl", "gedit", "kate"];
    let mut started = false;
    for cand in candidates {
        if Command::new(cand).arg(p).spawn().is_ok() {
            started = true;
            break;
        }
    }

    if !started {
        return Err(format!("Editor '{editor}' tidak ditemukan atau gagal dijalankan"));
    }

    Ok(())
}
