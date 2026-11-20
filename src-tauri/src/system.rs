use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn command_no_window<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = Command::new(program);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(program)
    }
}

fn resolve_file_manager_target(path: &str) -> Result<(std::path::PathBuf, bool), String> {
    let p = Path::new(path.trim());
    if p.as_os_str().is_empty() {
        return Err("Path tidak ditemukan".into());
    }
    if p.exists() {
        return Ok((p.to_path_buf(), p.is_file()));
    }
    if let Some(existing_parent) = p.ancestors().skip(1).find(|a| !a.as_os_str().is_empty() && a.exists()) {
        return Ok((existing_parent.to_path_buf(), false));
    }
    Err("Path tidak ditemukan".into())
}

#[cfg(target_os = "windows")]
fn format_windows_explorer_select_arg(p: &Path) -> Option<String> {
    let normalized = p.to_string_lossy().replace('/', "\\");
    if normalized.is_empty() || normalized.contains('"') {
        return None;
    }
    Some(format!("/select,\"{normalized}\""))
}

pub fn open_in_file_manager(path: &str) -> Result<(), String> {
    let (target, is_file) = resolve_file_manager_target(path)?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        if is_file {
            if let Some(raw) = format_windows_explorer_select_arg(&target) {
                Command::new("explorer")
                    .raw_arg(raw)
                    .spawn()
                    .map_err(|e| format!("Gagal membuka file manager: {e}"))?;
                return Ok(());
            }
        }
        Command::new("explorer")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Gagal membuka file manager: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("open");
        if is_file {
            cmd.arg("-R");
        }
        cmd.arg(&target)
            .spawn()
            .map_err(|e| format!("Gagal membuka file manager: {e}"))?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let dir = if is_file {
            target.parent().unwrap_or(&target)
        } else {
            &target
        };
        Command::new("xdg-open")
            .arg(dir)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableEditor {
    pub id: String,
    pub label: String,
    pub path: Option<String>,
    pub is_installed: bool,
    pub is_detected: bool,
    pub supports_folders: bool,
}

pub fn find_executable(name_or_path: &str) -> Option<PathBuf> {
    let p = Path::new(name_or_path);
    if p.is_absolute() {
        if p.is_file() {
            return Some(p.to_path_buf());
        }
        return None;
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name_or_path);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(target_os = "windows")]
            {
                let lower = name_or_path.to_ascii_lowercase();
                if !lower.ends_with(".exe") && !lower.ends_with(".cmd") && !lower.ends_with(".bat") {
                    let with_exe = dir.join(format!("{name_or_path}.exe"));
                    if with_exe.is_file() {
                        return Some(with_exe);
                    }
                    let with_cmd = dir.join(format!("{name_or_path}.cmd"));
                    if with_cmd.is_file() {
                        return Some(with_cmd);
                    }
                    let with_bat = dir.join(format!("{name_or_path}.bat"));
                    if with_bat.is_file() {
                        return Some(with_bat);
                    }
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn find_jetbrains_exe(prefix: &str, exe_name: &str) -> Option<PathBuf> {
    if let Some(prog_files) = std::env::var_os("ProgramFiles").map(PathBuf::from) {
        let jb_dir = prog_files.join("JetBrains");
        if jb_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&jb_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                    if path.is_dir() && name.starts_with(prefix) {
                        let exe = path.join("bin").join(exe_name);
                        if exe.is_file() {
                            return Some(exe);
                        }
                    }
                }
            }
        }
    }
    None
}

fn probe_editor_path(id: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let local_app = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let prog_files = std::env::var_os("ProgramFiles").map(PathBuf::from);
        let prog_files_x86 = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from);

        match id {
            "antigravity" | "antigravity-ide" | "agy" => {
                if let Some(p) = find_executable("antigravity-ide.cmd")
                    .or_else(|| find_executable("antigravity-ide.exe"))
                    .or_else(|| find_executable("antigravity-ide"))
                    .or_else(|| find_executable("antigravity.exe"))
                    .or_else(|| find_executable("antigravity"))
                    .or_else(|| find_executable("agy.cmd"))
                    .or_else(|| find_executable("agy.exe"))
                    .or_else(|| find_executable("agy"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c1 = d.join(r"Programs\Antigravity IDE\bin\antigravity-ide.cmd");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Programs\Antigravity IDE\Antigravity IDE.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                    let c3 = d.join(r"Programs\antigravity\Antigravity.exe");
                    if c3.is_file() {
                        return Some(c3);
                    }
                }
                if let Some(d) = &prog_files {
                    let c1 = d.join(r"Antigravity IDE\bin\antigravity-ide.cmd");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Antigravity IDE\Antigravity IDE.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                    let c3 = d.join(r"antigravity\Antigravity.exe");
                    if c3.is_file() {
                        return Some(c3);
                    }
                }
            }
            "code" => {
                if let Some(p) = find_executable("code.cmd")
                    .or_else(|| find_executable("code.exe"))
                    .or_else(|| find_executable("code"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c1 = d.join(r"Programs\Microsoft VS Code\bin\code.cmd");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Programs\Microsoft VS Code\Code.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                }
                if let Some(d) = &prog_files {
                    let c1 = d.join(r"Microsoft VS Code\bin\code.cmd");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Microsoft VS Code\Code.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                }
            }
            "cursor" => {
                if let Some(p) = find_executable("cursor.cmd")
                    .or_else(|| find_executable("cursor.exe"))
                    .or_else(|| find_executable("cursor"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c1 = d.join(r"Programs\cursor\resources\app\bin\cursor.cmd");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Programs\cursor\Cursor.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                }
                if let Some(d) = &prog_files {
                    let c1 = d.join(r"cursor\Cursor.exe");
                    if c1.is_file() {
                        return Some(c1);
                    }
                }
            }
            "windsurf" => {
                if let Some(p) = find_executable("windsurf.cmd")
                    .or_else(|| find_executable("windsurf.exe"))
                    .or_else(|| find_executable("windsurf"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c1 = d.join(r"Programs\Windsurf\bin\windsurf.cmd");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Programs\Windsurf\Windsurf.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                }
            }
            "zed" => {
                if let Some(p) = find_executable("zed.exe").or_else(|| find_executable("zed")) {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c1 = d.join(r"Programs\Zed\bin\Zed.exe");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Programs\Zed\Zed.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                }
            }
            "subl" => {
                if let Some(p) = find_executable("subl.exe")
                    .or_else(|| find_executable("sublime_text.exe"))
                    .or_else(|| find_executable("subl"))
                {
                    return Some(p);
                }
                if let Some(d) = &prog_files {
                    let c1 = d.join(r"Sublime Text\subl.exe");
                    if c1.is_file() {
                        return Some(c1);
                    }
                    let c2 = d.join(r"Sublime Text\sublime_text.exe");
                    if c2.is_file() {
                        return Some(c2);
                    }
                    let c3 = d.join(r"Sublime Text 3\subl.exe");
                    if c3.is_file() {
                        return Some(c3);
                    }
                }
                if let Some(d) = &prog_files_x86 {
                    let c1 = d.join(r"Sublime Text\subl.exe");
                    if c1.is_file() {
                        return Some(c1);
                    }
                }
            }
            "notepad++" => {
                if let Some(p) = find_executable("notepad++.exe").or_else(|| find_executable("notepad++")) {
                    return Some(p);
                }
                if let Some(d) = &prog_files {
                    let c = d.join(r"Notepad++\notepad++.exe");
                    if c.is_file() {
                        return Some(c);
                    }
                }
                if let Some(d) = &prog_files_x86 {
                    let c = d.join(r"Notepad++\notepad++.exe");
                    if c.is_file() {
                        return Some(c);
                    }
                }
            }
            "idea" => {
                if let Some(p) = find_executable("idea.cmd")
                    .or_else(|| find_executable("idea64.exe"))
                    .or_else(|| find_executable("idea"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c = d.join(r"JetBrains\Toolbox\scripts\idea.cmd");
                    if c.is_file() {
                        return Some(c);
                    }
                }
                if let Some(p) = find_jetbrains_exe("intellij idea", "idea64.exe") {
                    return Some(p);
                }
            }
            "webstorm" => {
                if let Some(p) = find_executable("webstorm.cmd")
                    .or_else(|| find_executable("webstorm64.exe"))
                    .or_else(|| find_executable("webstorm"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c = d.join(r"JetBrains\Toolbox\scripts\webstorm.cmd");
                    if c.is_file() {
                        return Some(c);
                    }
                }
                if let Some(p) = find_jetbrains_exe("webstorm", "webstorm64.exe") {
                    return Some(p);
                }
            }
            "pycharm" => {
                if let Some(p) = find_executable("pycharm.cmd")
                    .or_else(|| find_executable("pycharm64.exe"))
                    .or_else(|| find_executable("pycharm"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c = d.join(r"JetBrains\Toolbox\scripts\pycharm.cmd");
                    if c.is_file() {
                        return Some(c);
                    }
                }
                if let Some(p) = find_jetbrains_exe("pycharm", "pycharm64.exe") {
                    return Some(p);
                }
            }
            "rustrover" => {
                if let Some(p) = find_executable("rustrover.cmd")
                    .or_else(|| find_executable("rustrover64.exe"))
                    .or_else(|| find_executable("rustrover"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c = d.join(r"JetBrains\Toolbox\scripts\rustrover.cmd");
                    if c.is_file() {
                        return Some(c);
                    }
                }
                if let Some(p) = find_jetbrains_exe("rustrover", "rustrover64.exe") {
                    return Some(p);
                }
            }
            "goland" => {
                if let Some(p) = find_executable("goland.cmd")
                    .or_else(|| find_executable("goland64.exe"))
                    .or_else(|| find_executable("goland"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c = d.join(r"JetBrains\Toolbox\scripts\goland.cmd");
                    if c.is_file() {
                        return Some(c);
                    }
                }
                if let Some(p) = find_jetbrains_exe("goland", "goland64.exe") {
                    return Some(p);
                }
            }
            "codium" => {
                if let Some(p) = find_executable("codium.cmd")
                    .or_else(|| find_executable("codium.exe"))
                    .or_else(|| find_executable("codium"))
                {
                    return Some(p);
                }
                if let Some(d) = &local_app {
                    let c = d.join(r"Programs\VSCodium\VSCodium.exe");
                    if c.is_file() {
                        return Some(c);
                    }
                }
                if let Some(d) = &prog_files {
                    let c = d.join(r"VSCodium\VSCodium.exe");
                    if c.is_file() {
                        return Some(c);
                    }
                }
            }
            "nvim" => {
                return find_executable("nvim.exe")
                    .or_else(|| find_executable("nvim-qt.exe"))
                    .or_else(|| find_executable("nvim"));
            }
            "notepad" => {
                let default_np = PathBuf::from(r"C:\Windows\system32\notepad.exe");
                if default_np.is_file() {
                    return Some(default_np);
                }
                return find_executable("notepad.exe");
            }
            _ => {}
        }
        None
    }

    #[cfg(target_os = "macos")]
    {
        match id {
            "antigravity" | "antigravity-ide" | "agy" => {
                if let Some(p) = find_executable("antigravity-ide")
                    .or_else(|| find_executable("antigravity"))
                    .or_else(|| find_executable("agy"))
                {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/Antigravity IDE.app");
                if a1.exists() {
                    return Some(a1);
                }
                let a2 = PathBuf::from("/Applications/Antigravity.app");
                if a2.exists() {
                    return Some(a2);
                }
                if let Some(home) = std::env::var_os("HOME") {
                    let a3 = PathBuf::from(home).join("Applications/Antigravity IDE.app");
                    if a3.exists() {
                        return Some(a3);
                    }
                }
            }
            "code" => {
                if let Some(p) = find_executable("code") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/Visual Studio Code.app");
                if a1.exists() {
                    return Some(a1);
                }
                if let Some(home) = std::env::var_os("HOME") {
                    let a2 = PathBuf::from(home).join("Applications/Visual Studio Code.app");
                    if a2.exists() {
                        return Some(a2);
                    }
                }
            }
            "cursor" => {
                if let Some(p) = find_executable("cursor") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/Cursor.app");
                if a1.exists() {
                    return Some(a1);
                }
                if let Some(home) = std::env::var_os("HOME") {
                    let a2 = PathBuf::from(home).join("Applications/Cursor.app");
                    if a2.exists() {
                        return Some(a2);
                    }
                }
            }
            "windsurf" => {
                if let Some(p) = find_executable("windsurf") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/Windsurf.app");
                if a1.exists() {
                    return Some(a1);
                }
            }
            "zed" => {
                if let Some(p) = find_executable("zed") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/Zed.app");
                if a1.exists() {
                    return Some(a1);
                }
            }
            "subl" => {
                if let Some(p) = find_executable("subl") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/Sublime Text.app");
                if a1.exists() {
                    return Some(a1);
                }
            }
            "idea" => {
                if let Some(p) = find_executable("idea") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/IntelliJ IDEA.app");
                if a1.exists() {
                    return Some(a1);
                }
                let a2 = PathBuf::from("/Applications/IntelliJ IDEA CE.app");
                if a2.exists() {
                    return Some(a2);
                }
            }
            "webstorm" => {
                if let Some(p) = find_executable("webstorm") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/WebStorm.app");
                if a1.exists() {
                    return Some(a1);
                }
            }
            "pycharm" => {
                if let Some(p) = find_executable("pycharm") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/PyCharm.app");
                if a1.exists() {
                    return Some(a1);
                }
                let a2 = PathBuf::from("/Applications/PyCharm CE.app");
                if a2.exists() {
                    return Some(a2);
                }
            }
            "codium" => {
                if let Some(p) = find_executable("codium") {
                    return Some(p);
                }
                let a1 = PathBuf::from("/Applications/VSCodium.app");
                if a1.exists() {
                    return Some(a1);
                }
            }
            "nvim" => {
                return find_executable("nvim").or_else(|| find_executable("nvim-qt"));
            }
            _ => {}
        }
        None
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        match id {
            "antigravity" | "antigravity-ide" | "agy" => find_executable("antigravity-ide")
                .or_else(|| find_executable("antigravity"))
                .or_else(|| find_executable("agy")),
            "code" => find_executable("code"),
            "cursor" => find_executable("cursor"),
            "windsurf" => find_executable("windsurf"),
            "zed" => find_executable("zeditor").or_else(|| find_executable("zed")),
            "subl" => find_executable("subl").or_else(|| find_executable("sublime_text")),
            "gedit" => find_executable("gnome-text-editor").or_else(|| find_executable("gedit")),
            "kate" => find_executable("kate"),
            "idea" => find_executable("idea"),
            "webstorm" => find_executable("webstorm"),
            "pycharm" => find_executable("pycharm"),
            "rustrover" => find_executable("rustrover"),
            "goland" => find_executable("goland"),
            "codium" => find_executable("codium"),
            "nvim" => find_executable("nvim"),
            _ => None,
        }
    }
}

pub fn detect_available_editors() -> Vec<AvailableEditor> {
    let mut detected = Vec::new();

    // 1. System Default
    #[cfg(target_os = "windows")]
    let (sys_supports_folders, sys_path) = if let Some(txt_exe) = resolve_windows_txt_default_exe() {
        (is_folder_capable_editor_exe(&txt_exe), Some(txt_exe))
    } else {
        (false, Some(r"C:\Windows\system32\notepad.exe".to_string()))
    };

    #[cfg(not(target_os = "windows"))]
    let (sys_supports_folders, sys_path) = (true, None);

    detected.push(AvailableEditor {
        id: "system".into(),
        label: "Default Aplikasi Sistem".into(),
        path: sys_path,
        is_installed: true,
        is_detected: true,
        supports_folders: sys_supports_folders,
    });

    struct PresetSpec {
        id: &'static str,
        label: &'static str,
        supports_folders: bool,
    }

    let presets = [
        PresetSpec { id: "antigravity", label: "Antigravity IDE", supports_folders: true },
        PresetSpec { id: "code", label: "Visual Studio Code", supports_folders: true },
        PresetSpec { id: "cursor", label: "Cursor", supports_folders: true },
        PresetSpec { id: "windsurf", label: "Windsurf", supports_folders: true },
        PresetSpec { id: "zed", label: "Zed", supports_folders: true },
        PresetSpec { id: "subl", label: "Sublime Text", supports_folders: true },
        PresetSpec { id: "notepad++", label: "Notepad++", supports_folders: false },
        PresetSpec { id: "idea", label: "IntelliJ IDEA", supports_folders: true },
        PresetSpec { id: "webstorm", label: "WebStorm", supports_folders: true },
        PresetSpec { id: "pycharm", label: "PyCharm", supports_folders: true },
        PresetSpec { id: "rustrover", label: "RustRover", supports_folders: true },
        PresetSpec { id: "goland", label: "GoLand", supports_folders: true },
        PresetSpec { id: "codium", label: "VSCodium", supports_folders: true },
        PresetSpec { id: "nvim", label: "Neovim", supports_folders: true },
        PresetSpec { id: "notepad", label: "Notepad", supports_folders: false },
        PresetSpec { id: "gedit", label: "GNOME Text Editor / gedit", supports_folders: false },
        PresetSpec { id: "kate", label: "Kate", supports_folders: false },
    ];

    for spec in presets {
        let found_path = probe_editor_path(spec.id);
        let is_inst = found_path.is_some();
        let path_str = found_path.map(|p| p.to_string_lossy().to_string());

        detected.push(AvailableEditor {
            id: spec.id.into(),
            label: spec.label.into(),
            path: path_str,
            is_installed: is_inst,
            is_detected: is_inst,
            supports_folders: spec.supports_folders,
        });
    }

    // Keep System Default at index 0, sort the rest so that installed ones come first
    let mut rest = detected.split_off(1);
    rest.sort_by(|a, b| match (a.is_installed, b.is_installed) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    });
    detected.extend(rest);

    detected
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
            | "antigravity.exe"
            | "antigravity ide.exe"
            | "antigravity-ide.cmd"
            | "agy.cmd"
            | "agy.exe"
            | "zed.exe"
            | "subl.exe"
            | "sublime_text.exe"
            | "notepad++.exe"
            | "codium.exe"
            | "devenv.exe"
            | "idea64.exe"
            | "webstorm64.exe"
            | "pycharm64.exe"
            | "rustrover64.exe"
            | "goland64.exe"
            | "clion64.exe"
            | "phpstorm64.exe"
            | "studio64.exe"
    )
}

#[cfg(target_os = "windows")]
fn windows_known_editor_paths(editor: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let local_app = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from);
    let prog_files = std::env::var_os("ProgramFiles").map(std::path::PathBuf::from);
    let prog_files_x86 = std::env::var_os("ProgramFiles(x86)").map(std::path::PathBuf::from);

    match editor.to_ascii_lowercase().as_str() {
        "antigravity" | "antigravity-ide" | "agy" => {
            if let Some(d) = &local_app {
                out.push(d.join(r"Programs\Antigravity IDE\bin\antigravity-ide.cmd"));
                out.push(d.join(r"Programs\Antigravity IDE\Antigravity IDE.exe"));
                out.push(d.join(r"Programs\antigravity\Antigravity.exe"));
            }
            if let Some(d) = &prog_files {
                out.push(d.join(r"Antigravity IDE\bin\antigravity-ide.cmd"));
                out.push(d.join(r"Antigravity IDE\Antigravity IDE.exe"));
                out.push(d.join(r"antigravity\Antigravity.exe"));
            }
        }
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
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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
            // Smart auto-fallback for folders: if default text editor cannot open folders,
            // try to launch the first installed folder-capable editor (e.g. Zed, VS Code, Cursor)
            for editor in detect_available_editors() {
                if editor.supports_folders && editor.is_installed && editor.id != "system" {
                    if let Some(ref path_str) = editor.path {
                        let candidate = PathBuf::from(path_str);
                        if candidate.is_file() {
                            let mut cmd = Command::new(&candidate);
                            cmd.arg(p);
                            if path_str.to_ascii_lowercase().ends_with(".cmd")
                                || path_str.to_ascii_lowercase().ends_with(".bat")
                            {
                                cmd.creation_flags(CREATE_NO_WINDOW);
                            }
                            if cmd.spawn().is_ok() {
                                return Ok(());
                            }
                        }
                    }
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

        // 1. If path or executable with directory separator was explicitly provided
        if editor.contains('/') || editor.contains('\\') {
            let direct = Path::new(editor);
            if direct.is_file() {
                let mut cmd = Command::new(direct);
                cmd.arg(p);
                if editor.to_ascii_lowercase().ends_with(".cmd")
                    || editor.to_ascii_lowercase().ends_with(".bat")
                {
                    cmd.creation_flags(CREATE_NO_WINDOW);
                }
                if cmd.spawn().is_ok() {
                    return Ok(());
                }
            } else if Command::new(editor).arg(p).spawn().is_ok() {
                return Ok(());
            }
            return Err(format!(
                "Editor kustom '{editor}' tidak ditemukan atau gagal dijalankan. Periksa pengaturan Editor Eksternal."
            ));
        }

        // 2. Try detected editor path from discovery engine
        let available = detect_available_editors();
        if let Some(matched) = available.iter().find(|e| e.id.eq_ignore_ascii_case(editor)) {
            if let Some(ref path_str) = matched.path {
                let candidate = PathBuf::from(path_str);
                if candidate.is_file() {
                    let mut cmd = Command::new(&candidate);
                    cmd.arg(p);
                    if path_str.to_ascii_lowercase().ends_with(".cmd")
                        || path_str.to_ascii_lowercase().ends_with(".bat")
                    {
                        cmd.creation_flags(CREATE_NO_WINDOW);
                    }
                    if cmd.spawn().is_ok() {
                        return Ok(());
                    }
                }
            }
        }

        // 3. Fallbacks: direct command, .cmd, .exe, windows_known_editor_paths
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

        let installed_labels: Vec<String> = available
            .iter()
            .filter(|e| e.is_installed && e.id != "system" && e.id != "notepad")
            .map(|e| e.label.clone())
            .collect();
        let hint = if !installed_labels.is_empty() {
            format!(" Editor yang terdeteksi di sistem: {}.", installed_labels.join(", "))
        } else {
            String::new()
        };

        return Err(format!(
            "Editor '{editor}' tidak ditemukan atau gagal dijalankan.{hint} Periksa pengaturan Editor Eksternal."
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
            let available = detect_available_editors();
            if let Some(matched) = available.iter().find(|e| e.id.eq_ignore_ascii_case(editor)) {
                if let Some(ref path_str) = matched.path {
                    if path_str.ends_with(".app") {
                        if Command::new("open").args(["-a", path_str]).arg(p).spawn().is_ok() {
                            return Ok(());
                        }
                    } else if Command::new(path_str).arg(p).spawn().is_ok() {
                        return Ok(());
                    }
                }
            }
            if Command::new(editor).arg(p).spawn().is_ok() {
                return Ok(());
            }
            let mac_app = match editor.to_ascii_lowercase().as_str() {
                "antigravity" | "antigravity-ide" | "agy" => Some("Antigravity IDE"),
                "code" => Some("Visual Studio Code"),
                "cursor" => Some("Cursor"),
                "windsurf" => Some("Windsurf"),
                "zed" => Some("Zed"),
                "subl" => Some("Sublime Text"),
                "idea" => Some("IntelliJ IDEA"),
                "webstorm" => Some("WebStorm"),
                "pycharm" => Some("PyCharm"),
                "codium" => Some("VSCodium"),
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
        let available = detect_available_editors();
        if let Some(matched) = available.iter().find(|e| e.id.eq_ignore_ascii_case(editor)) {
            if let Some(ref path_str) = matched.path {
                if Command::new(path_str).arg(p).spawn().is_ok() {
                    return Ok(());
                }
            }
        }
        if Command::new(editor).arg(p).spawn().is_ok() {
            return Ok(());
        }
        let linux_fallback = match editor {
            "gedit" => Some("gnome-text-editor"),
            "zed" => Some("zeditor"),
            "subl" => Some("sublime_text"),
            "antigravity" | "agy" => Some("antigravity-ide"),
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

    #[test]
    fn resolves_existing_file_dir_and_deleted_file_ancestor() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("forge-reveal-test-{nanos}"));
        let sub = root.join("nested");
        let file = sub.join("example.txt");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(&file, "hello").unwrap();

        let (t_file, is_file) = resolve_file_manager_target(file.to_str().unwrap()).unwrap();
        assert_eq!(t_file, file);
        assert!(is_file);

        let (t_dir, is_dir_file) = resolve_file_manager_target(sub.to_str().unwrap()).unwrap();
        assert_eq!(t_dir, sub);
        assert!(!is_dir_file);

        let missing = sub.join("deleted.txt");
        let (t_missing, is_missing_file) = resolve_file_manager_target(missing.to_str().unwrap()).unwrap();
        assert_eq!(t_missing, sub);
        assert!(!is_missing_file);

        assert!(resolve_file_manager_target("").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn formats_windows_explorer_select_arg() {
        let arg = format_windows_explorer_select_arg(Path::new("C:/Development/my repo/file.ts")).unwrap();
        assert_eq!(arg, r#"/select,"C:\Development\my repo\file.ts""#);
    }

    #[test]
    fn test_detect_available_editors_returns_system_and_presets() {
        let editors = detect_available_editors();
        assert!(!editors.is_empty(), "Daftar editor tidak boleh kosong");
        assert_eq!(editors[0].id, "system", "Elemen pertama harus selalu system");
        assert!(editors[0].is_installed);
        assert!(editors.iter().any(|e| e.id == "antigravity"));
        assert!(editors.iter().any(|e| e.id == "code"));
        assert!(editors.iter().any(|e| e.id == "zed"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_detect_available_editors_identifies_installed_antigravity() {
        let editors = detect_available_editors();
        if let Some(ag) = editors.iter().find(|e| e.id == "antigravity") {
            if ag.is_installed {
                assert!(ag.path.is_some());
                assert!(ag.supports_folders);
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_detect_available_editors_identifies_installed_zed() {
        let editors = detect_available_editors();
        if let Some(zed) = editors.iter().find(|e| e.id == "zed") {
            if zed.is_installed {
                assert!(zed.path.is_some());
                assert!(zed.supports_folders);
            }
        }
    }
}
