use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const GITHUB_REPO_OWNER: &str = "machfudn";
pub const GITHUB_REPO_NAME: &str = "forge-desktop";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    Auto,
    Stable,
    Beta,
}

impl UpdateChannel {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).unwrap_or("auto").to_ascii_lowercase().as_str() {
            "stable" => Self::Stable,
            "beta" | "prerelease" => Self::Beta,
            _ => Self::Auto,
        }
    }

    pub fn resolve(self, current_version: &str) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Auto => {
                if current_version.contains('-') {
                    "beta"
                } else {
                    "stable"
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKind {
    WindowsPortable,
    WindowsNsis,
    WindowsMsi,
    LinuxAppImage,
    LinuxDeb,
    LinuxRpm,
    LinuxPortable,
    MacOsApp,
    MacOsInstalled,
}

impl PackageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WindowsPortable => "windows_portable",
            Self::WindowsNsis => "windows_nsis",
            Self::WindowsMsi => "windows_msi",
            Self::LinuxAppImage => "linux_appimage",
            Self::LinuxDeb => "linux_deb",
            Self::LinuxRpm => "linux_rpm",
            Self::LinuxPortable => "linux_portable",
            Self::MacOsApp => "macos_app",
            Self::MacOsInstalled => "macos_installed",
        }
    }

    pub fn install_mode(self) -> &'static str {
        match self {
            Self::WindowsPortable
            | Self::LinuxAppImage
            | Self::LinuxPortable
            | Self::MacOsApp => "portable",
            Self::WindowsNsis
            | Self::WindowsMsi
            | Self::LinuxDeb
            | Self::LinuxRpm
            | Self::MacOsInstalled => "installed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub channel: String,
    pub is_prerelease: bool,
    pub install_mode: String,
    pub package_kind: String,
    pub can_self_update: bool,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: Option<u64>,
    pub sha256: Option<String>,
    pub release_notes: Option<String>,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRuntimeInfo {
    pub current_version: String,
    pub os: String,
    pub arch: String,
    pub install_mode: String,
    pub package_kind: String,
    pub default_channel: String,
}

pub fn get_app_runtime_info() -> AppRuntimeInfo {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let kind = detect_package_kind();
    AppRuntimeInfo {
        default_channel: UpdateChannel::Auto.resolve(&current_version).to_string(),
        current_version,
        os: std::env::consts::OS.to_string(),
        arch: current_arch().to_string(),
        install_mode: kind.install_mode().to_string(),
        package_kind: kind.as_str().to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateApplyResult {
    pub launched: bool,
    pub restart_pending: bool,
    pub message: String,
    pub downloaded_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhReleaseAsset {
    pub name: String,
    pub url: Option<String>,
    pub browser_download_url: String,
    pub size: Option<u64>,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhRelease {
    pub tag_name: String,
    pub body: Option<String>,
    pub html_url: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<GhReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSemver {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<PreId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreId {
    Num(u64),
    Str(String),
}

impl Ord for PreId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (PreId::Num(a), PreId::Num(b)) => a.cmp(b),
            (PreId::Num(_), PreId::Str(_)) => std::cmp::Ordering::Less,
            (PreId::Str(_), PreId::Num(_)) => std::cmp::Ordering::Greater,
            (PreId::Str(a), PreId::Str(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for PreId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_semver(raw: &str) -> Option<ParsedSemver> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let without_build = trimmed.split('+').next()?.trim();
    if without_build.is_empty() {
        return None;
    }
    let (core, pre_raw) = match without_build.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (without_build, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let pre = match pre_raw {
        Some(p) if !p.is_empty() => p
            .split('.')
            .map(|seg| match seg.parse::<u64>() {
                Ok(n) => PreId::Num(n),
                Err(_) => PreId::Str(seg.to_ascii_lowercase()),
            })
            .collect(),
        _ => Vec::new(),
    };
    Some(ParsedSemver {
        major,
        minor,
        patch,
        pre,
    })
}

pub fn compare_semver(a: &str, b: &str) -> std::cmp::Ordering {
    match (parse_semver(a), parse_semver(b)) {
        (Some(pa), Some(pb)) => {
            let core_ord = (pa.major, pa.minor, pa.patch).cmp(&(pb.major, pb.minor, pb.patch));
            if core_ord != std::cmp::Ordering::Equal {
                return core_ord;
            }
            match (pa.pre.is_empty(), pb.pre.is_empty()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => {
                    let len = pa.pre.len().max(pb.pre.len());
                    for i in 0..len {
                        match (pa.pre.get(i), pb.pre.get(i)) {
                            (Some(x), Some(y)) => {
                                let ord = x.cmp(y);
                                if ord != std::cmp::Ordering::Equal {
                                    return ord;
                                }
                            }
                            (Some(_), None) => return std::cmp::Ordering::Greater,
                            (None, Some(_)) => return std::cmp::Ordering::Less,
                            (None, None) => break,
                        }
                    }
                    std::cmp::Ordering::Equal
                }
            }
        }
        _ => a.cmp(b),
    }
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    compare_semver(latest, current) == std::cmp::Ordering::Greater
}

pub fn current_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn detect_package_kind() -> PackageKind {
    let exe = std::env::current_exe().unwrap_or_default();
    detect_package_kind_for(
        std::env::consts::OS,
        &exe,
        std::env::var_os("APPIMAGE").is_some(),
    )
}

pub fn detect_package_kind_for(os: &str, exe: &Path, has_appimage_env: bool) -> PackageKind {
    match os {
        "windows" => {
            let exe_lower = exe.to_string_lossy().to_ascii_lowercase();
            let dir = exe.parent();
            let has_portable_marker = dir
                .map(|d| d.join(".portable").exists())
                .unwrap_or(false)
                || exe_lower.contains("portable");
            if has_portable_marker {
                return PackageKind::WindowsPortable;
            }
            let has_nsis_uninstaller = dir
                .map(|d| d.join("uninstall.exe").exists())
                .unwrap_or(false);
            if has_nsis_uninstaller {
                return PackageKind::WindowsNsis;
            }
            if exe_lower.contains("\\program files\\")
                || exe_lower.contains("\\program files (x86)\\")
            {
                return PackageKind::WindowsMsi;
            }
            if exe_lower.contains("\\appdata\\local\\programs\\")
                || exe_lower.contains("\\appdata\\local\\forge desktop\\")
            {
                return PackageKind::WindowsNsis;
            }
            PackageKind::WindowsPortable
        }
        "linux" => {
            if has_appimage_env {
                return PackageKind::LinuxAppImage;
            }
            let exe_str = exe.to_string_lossy();
            if exe_str.starts_with("/usr/") || exe_str.starts_with("/opt/") {
                if Command::new("dpkg")
                    .args(["-S", &exe_str])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    return PackageKind::LinuxDeb;
                }
                if Command::new("rpm")
                    .args(["-qf", &exe_str])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    return PackageKind::LinuxRpm;
                }
                if Path::new("/etc/debian_version").exists() {
                    return PackageKind::LinuxDeb;
                }
                return PackageKind::LinuxRpm;
            }
            PackageKind::LinuxPortable
        }
        "macos" => {
            let exe_str = exe.to_string_lossy();
            if exe_str.starts_with("/Applications/") {
                PackageKind::MacOsInstalled
            } else {
                PackageKind::MacOsApp
            }
        }
        _ => PackageKind::LinuxPortable,
    }
}

fn asset_matches_arch(name_lower: &str, arch: &str) -> bool {
    let is_arm = name_lower.contains("aarch64") || name_lower.contains("arm64");
    let is_x64 = name_lower.contains("x86_64")
        || name_lower.contains("x64")
        || name_lower.contains("amd64");
    match arch {
        "aarch64" => !is_x64,
        "x86_64" => !is_arm,
        _ => true,
    }
}

pub fn select_best_asset<'a>(
    assets: &'a [GhReleaseAsset],
    arch: &str,
    kind: PackageKind,
) -> Option<&'a GhReleaseAsset> {
    let candidates: Vec<&GhReleaseAsset> = assets
        .iter()
        .filter(|a| {
            let n = a.name.to_ascii_lowercase();
            !n.ends_with(".sig")
                && !n.ends_with(".asc")
                && n != "sha256sums"
                && !n.ends_with(".json")
                && asset_matches_arch(&n, arch)
        })
        .collect();

    let find_first = |pred: &dyn Fn(&str) -> bool| -> Option<&'a GhReleaseAsset> {
        candidates
            .iter()
            .copied()
            .find(|a| pred(&a.name.to_ascii_lowercase()))
    };

    match kind {
        PackageKind::WindowsPortable => find_first(&|n| n.ends_with("-portable.exe"))
            .or_else(|| find_first(&|n| n.ends_with("-portable.zip")))
            .or_else(|| find_first(&|n| n.contains("windows") && n.ends_with(".exe") && !n.ends_with("-setup.exe"))),
        PackageKind::WindowsNsis => find_first(&|n| n.ends_with("-setup.exe"))
            .or_else(|| find_first(&|n| n.ends_with(".msi"))),
        PackageKind::WindowsMsi => find_first(&|n| n.ends_with(".msi"))
            .or_else(|| find_first(&|n| n.ends_with("-setup.exe"))),
        PackageKind::LinuxAppImage => find_first(&|n| n.ends_with(".appimage")),
        PackageKind::LinuxPortable => find_first(&|n| n.contains("linux") && n.ends_with("-portable.tar.gz"))
            .or_else(|| find_first(&|n| n.ends_with(".appimage"))),
        PackageKind::LinuxDeb => find_first(&|n| n.ends_with(".deb"))
            .or_else(|| find_first(&|n| n.ends_with(".appimage"))),
        PackageKind::LinuxRpm => find_first(&|n| n.ends_with(".rpm"))
            .or_else(|| find_first(&|n| n.ends_with(".appimage"))),
        PackageKind::MacOsApp | PackageKind::MacOsInstalled => {
            find_first(&|n| n.ends_with(".app.tar.gz"))
                .or_else(|| find_first(&|n| n.ends_with(".dmg")))
        }
    }
}

pub fn pick_target_release<'a>(
    releases: &'a [GhRelease],
    resolved_channel: &str,
) -> Option<&'a GhRelease> {
    releases
        .iter()
        .filter(|r| !r.draft)
        .filter(|r| {
            if resolved_channel == "stable" {
                !r.prerelease && !r.tag_name.trim_start_matches(['v', 'V']).contains('-')
            } else {
                true
            }
        })
        .max_by(|a, b| compare_semver(&a.tag_name, &b.tag_name))
}

pub fn parse_sha256sums(raw: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let Some(hash) = parts.next() else {
            continue;
        };
        let Some(rest) = parts.next() else {
            continue;
        };
        let hash_clean = hash.trim().to_ascii_lowercase();
        if hash_clean.len() != 64 || !hash_clean.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let name = rest.trim().trim_start_matches('*').trim_start_matches("./");
        let base = Path::new(name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(name);
        if !base.is_empty() {
            map.insert(base.to_ascii_lowercase(), hash_clean);
        }
    }
    map
}

pub fn is_trusted_github_url(url: &str) -> bool {
    let trimmed = url.trim();
    let Some(rest) = trimmed.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("").to_ascii_lowercase();
    host == "github.com"
        || host == "api.github.com"
        || host.ends_with(".githubusercontent.com")
}

fn asset_sha256_from_digest(asset: &GhReleaseAsset) -> Option<String> {
    let raw = asset.digest.as_deref()?.trim();
    let hex = raw.strip_prefix("sha256:").unwrap_or(raw).to_ascii_lowercase();
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(hex)
    } else {
        None
    }
}

fn fetch_sha256sums(
    release: &GhRelease,
    token: Option<&str>,
) -> HashMap<String, String> {
    let Some(sums_asset) = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("SHA256SUMS"))
    else {
        return HashMap::new();
    };
    if !is_trusted_github_url(&sums_asset.browser_download_url) {
        return HashMap::new();
    }
    let tmp = std::env::temp_dir().join(format!("forge-sha256sums-{}.txt", std::process::id()));
    let res = download_asset_to_file(sums_asset, &tmp, token);
    let parsed = if res.is_ok() {
        fs::read_to_string(&tmp)
            .map(|s| parse_sha256sums(&s))
            .unwrap_or_default()
    } else {
        HashMap::new()
    };
    let _ = fs::remove_file(&tmp);
    parsed
}

fn download_asset_to_file(
    asset: &GhReleaseAsset,
    dest: &Path,
    token: Option<&str>,
) -> Result<(), String> {
    let (download_url, is_api_asset) = match (token, asset.url.as_deref()) {
        (Some(t), Some(api_url)) if !t.trim().is_empty() && is_trusted_github_url(api_url) => {
            (api_url, true)
        }
        _ => (asset.browser_download_url.as_str(), false),
    };
    if !is_trusted_github_url(download_url) {
        return Err("URL unduhan pembaruan tidak berasal dari domain GitHub tepercaya".into());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Gagal membuat folder unduhan pembaruan: {e}"))?;
    }

    let mut cmd = Command::new("curl");
    cmd.args([
        "-fsSL",
        "--retry",
        "2",
        "--connect-timeout",
        "20",
        "-K",
        "-",
        "-o",
    ])
    .arg(dest)
    .arg(download_url)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Gagal menjalankan curl untuk mengunduh pembaruan: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let mut cfg = format!(
            "header = \"User-Agent: ForgeDesktop/{}\"\n",
            env!("CARGO_PKG_VERSION")
        );
        if is_api_asset {
            cfg.push_str("header = \"Accept: application/octet-stream\"\n");
            if let Some(t) = token
                .map(str::trim)
                .filter(|s| !s.is_empty() && !s.contains(['\r', '\n', '"']))
            {
                cfg.push_str(&format!("header = \"Authorization: Bearer {t}\"\n"));
            }
        }
        let _ = stdin.write_all(cfg.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Gagal menunggu proses unduhan pembaruan: {e}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(dest);
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Unduhan pembaruan {} gagal.", asset.name)
        } else {
            format!("Unduhan pembaruan gagal: {err}")
        });
    }
    Ok(())
}

pub fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| format!("Gagal membuka file untuk verifikasi SHA-256: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| format!("Gagal membaca file saat verifikasi SHA-256: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}

pub fn verify_file_sha256(path: &Path, expected_hex: &str) -> Result<(), String> {
    let actual = compute_file_sha256(path)?;
    if !actual.eq_ignore_ascii_case(expected_hex.trim()) {
        return Err(format!(
            "Checksum SHA-256 tidak cocok (diharapkan {}, didapat {}).",
            expected_hex.trim(),
            actual
        ));
    }
    Ok(())
}

struct ResolvedCandidate {
    channel: String,
    release: Option<GhRelease>,
    asset: Option<GhReleaseAsset>,
    sha256: Option<String>,
}

fn resolve_release_candidate(
    channel: UpdateChannel,
    token: Option<&str>,
    package_kind: PackageKind,
) -> Result<ResolvedCandidate, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let resolved_channel = channel.resolve(&current).to_string();
    let url = format!(
        "https://api.github.com/repos/{GITHUB_REPO_OWNER}/{GITHUB_REPO_NAME}/releases?per_page=20"
    );
    let stdout = crate::git::github::github_api_request("GET", &url, None, token)?;
    if stdout.trim().is_empty() {
        return Ok(ResolvedCandidate {
            channel: resolved_channel,
            release: None,
            asset: None,
            sha256: None,
        });
    }
    let releases: Vec<GhRelease> = serde_json::from_str(&stdout)
        .map_err(|_| "Gagal membaca daftar rilis GitHub".to_string())?;
    let Some(target) = pick_target_release(&releases, &resolved_channel).cloned() else {
        return Ok(ResolvedCandidate {
            channel: resolved_channel,
            release: None,
            asset: None,
            sha256: None,
        });
    };
    let asset = select_best_asset(&target.assets, current_arch(), package_kind).cloned();
    let sha = if let Some(ref a) = asset {
        let sums = fetch_sha256sums(&target, token);
        sums.get(&a.name.to_ascii_lowercase())
            .cloned()
            .or_else(|| asset_sha256_from_digest(a))
    } else {
        None
    };
    Ok(ResolvedCandidate {
        channel: resolved_channel,
        release: Some(target),
        asset,
        sha256: sha,
    })
}

pub fn check_app_update(
    channel: UpdateChannel,
    token: Option<&str>,
) -> Result<AppUpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let package_kind = detect_package_kind();
    let ResolvedCandidate {
        channel: resolved_channel,
        release: release_opt,
        asset: asset_opt,
        sha256: sha_opt,
    } = resolve_release_candidate(channel, token, package_kind)?;
    let Some(release) = release_opt else {
        return Ok(AppUpdateInfo {
            current_version: current.clone(),
            latest_version: current,
            has_update: false,
            channel: resolved_channel,
            is_prerelease: false,
            install_mode: package_kind.install_mode().to_string(),
            package_kind: package_kind.as_str().to_string(),
            can_self_update: false,
            asset_name: None,
            asset_url: None,
            asset_size: None,
            sha256: None,
            release_notes: None,
            html_url: None,
        });
    };

    let latest_clean = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    let has_update = is_newer_version(&latest_clean, &current);
    let can_self_update = has_update && asset_opt.is_some();

    Ok(AppUpdateInfo {
        current_version: current,
        latest_version: latest_clean,
        has_update,
        channel: resolved_channel,
        is_prerelease: release.prerelease,
        install_mode: package_kind.install_mode().to_string(),
        package_kind: package_kind.as_str().to_string(),
        can_self_update,
        asset_name: asset_opt.as_ref().map(|a| a.name.clone()),
        asset_url: asset_opt.as_ref().map(|a| a.browser_download_url.clone()),
        asset_size: asset_opt.as_ref().and_then(|a| a.size),
        sha256: sha_opt,
        release_notes: release.body,
        html_url: release.html_url,
    })
}

fn ensure_dir_writable(dir: &Path) -> Result<(), String> {
    let probe = dir.join(format!(".forge-write-probe-{}", std::process::id()));
    fs::write(&probe, b"ok").map_err(|e| {
        format!(
            "Folder aplikasi tidak dapat ditulisi ({}): {e}. Pindahkan versi portable ke folder milik pengguna atau jalankan pembaruan dengan izin yang sesuai.",
            dir.display()
        )
    })?;
    let _ = fs::remove_file(&probe);
    Ok(())
}

#[cfg(target_os = "windows")]
fn find_first_exe_recursive(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_first_exe_recursive(&path) {
                return Some(found);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn apply_windows_update(
    kind: PackageKind,
    downloaded: &Path,
    stage_dir: &Path,
    asset_name: &str,
) -> Result<UpdateApplyResult, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Gagal menentukan lokasi executable saat ini: {e}"))?;
    let lower = asset_name.to_ascii_lowercase();
    let pid = std::process::id();

    let (mode, payload_path) = if kind == PackageKind::WindowsPortable && lower.ends_with(".zip") {
        let extract_dir = stage_dir.join("extracted");
        fs::create_dir_all(&extract_dir)
            .map_err(|e| format!("Gagal membuat folder ekstraksi portable: {e}"))?;
        let status = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force",
            ])
            .arg(downloaded)
            .arg(&extract_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("Gagal mengekstrak arsip portable: {e}"))?;
        if !status.success() {
            return Err("Ekstraksi arsip portable .zip gagal.".into());
        }
        let exe_inside = find_first_exe_recursive(&extract_dir)
            .ok_or_else(|| "File .exe tidak ditemukan di dalam arsip portable.".to_string())?;
        ("portable", exe_inside)
    } else if kind == PackageKind::WindowsPortable && lower.ends_with(".exe") && !lower.ends_with("-setup.exe") {
        ("portable", downloaded.to_path_buf())
    } else if lower.ends_with(".msi") {
        ("msi", downloaded.to_path_buf())
    } else {
        ("nsis", downloaded.to_path_buf())
    };

    if mode == "portable" {
        let exe_dir = current_exe
            .parent()
            .ok_or_else(|| "Folder executable tidak valid".to_string())?;
        ensure_dir_writable(exe_dir)?;
    }

    let script_path = stage_dir.join("apply-update.ps1");
    let script_body = r#"param(
  [int]$TargetPid,
  [string]$Mode,
  [string]$PayloadPath,
  [string]$CurrentExe,
  [string]$StageDir
)
$ErrorActionPreference = 'Stop'
for ($i = 0; $i -lt 80; $i++) {
  if (-not (Get-Process -Id $TargetPid -ErrorAction SilentlyContinue)) { break }
  Start-Sleep -Milliseconds 250
}
if ($Mode -eq 'portable') {
  $bak = "$CurrentExe.bak"
  if (Test-Path -LiteralPath $bak) { Remove-Item -LiteralPath $bak -Force -ErrorAction SilentlyContinue }
  $movedOld = $false
  try {
    Move-Item -LiteralPath $CurrentExe -Destination $bak -Force
    $movedOld = $true
    Copy-Item -LiteralPath $PayloadPath -Destination $CurrentExe -Force
    Remove-Item -LiteralPath $bak -Force -ErrorAction SilentlyContinue
  } catch {
    if ($movedOld -and (Test-Path -LiteralPath $bak) -and -not (Test-Path -LiteralPath $CurrentExe)) {
      Move-Item -LiteralPath $bak -Destination $CurrentExe -Force -ErrorAction SilentlyContinue
    }
    throw
  }
  Start-Process -FilePath $CurrentExe
} elseif ($Mode -eq 'msi') {
  $proc = Start-Process -FilePath "msiexec.exe" -ArgumentList @("/i", $PayloadPath, "/passive", "/norestart") -Wait -PassThru
  if ($proc.ExitCode -eq 0 -and (Test-Path -LiteralPath $CurrentExe)) {
    Start-Process -FilePath $CurrentExe
  }
} else {
  $proc = Start-Process -FilePath $PayloadPath -ArgumentList @("/P", "/UPDATE") -Wait -PassThru
  if ($proc.ExitCode -eq 0 -and (Test-Path -LiteralPath $CurrentExe)) {
    Start-Process -FilePath $CurrentExe
  }
}
Remove-Item -LiteralPath $StageDir -Recurse -Force -ErrorAction SilentlyContinue
"#;
    fs::write(&script_path, script_body)
        .map_err(|e| format!("Gagal menyiapkan skrip pembaruan Windows: {e}"))?;

    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .arg(pid.to_string())
        .arg(mode)
        .arg(&payload_path)
        .arg(&current_exe)
        .arg(stage_dir)
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Gagal menjalankan proses pembaruan Windows: {e}"))?;

    let msg = if mode == "portable" {
        "Pembaruan portable terverifikasi. Aplikasi akan ditutup sebentar untuk mengganti executable dan memulai ulang otomatis.".into()
    } else {
        "Installer pembaruan terverifikasi. Aplikasi akan ditutup sementara untuk memperbarui instalasi dan memulai ulang otomatis.".into()
    };

    Ok(UpdateApplyResult {
        launched: true,
        restart_pending: true,
        message: msg,
        downloaded_path: Some(downloaded.display().to_string()),
    })
}

#[cfg(target_os = "linux")]
fn find_file_named_recursive(dir: &Path, target_name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_named_recursive(&path, target_name) {
                return Some(found);
            }
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == target_name)
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn apply_linux_update(
    kind: PackageKind,
    downloaded: &Path,
    stage_dir: &Path,
    asset_name: &str,
) -> Result<UpdateApplyResult, String> {
    use std::os::unix::fs::PermissionsExt;
    let lower = asset_name.to_ascii_lowercase();

    if kind == PackageKind::LinuxAppImage || lower.ends_with(".appimage") {
        let target_path = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_exe().unwrap_or_default());
        let parent = target_path
            .parent()
            .ok_or_else(|| "Folder AppImage tidak valid".to_string())?;
        ensure_dir_writable(parent)?;

        let tmp_next = parent.join(format!(".forge-update-{}.AppImage.tmp", std::process::id()));
        fs::copy(downloaded, &tmp_next)
            .map_err(|e| format!("Gagal menyalin AppImage pembaruan: {e}"))?;
        fs::set_permissions(&tmp_next, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Gagal mengatur izin eksekusi AppImage: {e}"))?;
        fs::rename(&tmp_next, &target_path)
            .map_err(|e| format!("Gagal mengganti AppImage: {e}"))?;
        let _ = fs::remove_dir_all(stage_dir);

        Command::new(&target_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("AppImage diperbarui, tetapi gagal memulai ulang: {e}"))?;

        return Ok(UpdateApplyResult {
            launched: true,
            restart_pending: true,
            message: "AppImage berhasil diperbarui. Memulai ulang versi baru...".into(),
            downloaded_path: Some(target_path.display().to_string()),
        });
    }

    if kind == PackageKind::LinuxPortable && lower.ends_with(".tar.gz") {
        let current_exe =
            std::env::current_exe().map_err(|e| format!("Gagal menentukan lokasi binary: {e}"))?;
        let parent = current_exe
            .parent()
            .ok_or_else(|| "Folder binary tidak valid".to_string())?;
        ensure_dir_writable(parent)?;

        let extract_dir = stage_dir.join("extracted");
        fs::create_dir_all(&extract_dir)
            .map_err(|e| format!("Gagal membuat folder ekstraksi: {e}"))?;
        let status = Command::new("tar")
            .args(["-xzf"])
            .arg(downloaded)
            .arg("-C")
            .arg(&extract_dir)
            .status()
            .map_err(|e| format!("Gagal menjalankan tar: {e}"))?;
        if !status.success() {
            return Err("Ekstraksi arsip portable .tar.gz gagal.".into());
        }
        let new_bin = find_file_named_recursive(&extract_dir, "forge-desktop")
            .ok_or_else(|| "Binary forge-desktop tidak ditemukan di dalam arsip.".to_string())?;
        let tmp_next = parent.join(format!(".forge-update-{}.tmp", std::process::id()));
        fs::copy(&new_bin, &tmp_next)
            .map_err(|e| format!("Gagal menyalin binary baru: {e}"))?;
        fs::set_permissions(&tmp_next, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Gagal mengatur izin eksekusi: {e}"))?;
        fs::rename(&tmp_next, &current_exe)
            .map_err(|e| format!("Gagal mengganti binary: {e}"))?;
        let _ = fs::remove_dir_all(stage_dir);

        Command::new(&current_exe)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Binary diperbarui, tetapi gagal memulai ulang: {e}"))?;

        return Ok(UpdateApplyResult {
            launched: true,
            restart_pending: true,
            message: "Binary portable Linux berhasil diperbarui. Memulai ulang...".into(),
            downloaded_path: Some(current_exe.display().to_string()),
        });
    }

    // Installed Linux (.deb or .rpm)
    let has_pkexec = Command::new("sh")
        .args(["-c", "command -v pkexec >/dev/null 2>&1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if has_pkexec {
        let output = if lower.ends_with(".deb") {
            Command::new("pkexec")
                .args(["apt-get", "install", "-y"])
                .arg(downloaded)
                .output()
        } else {
            Command::new("pkexec")
                .args(["dnf", "install", "-y"])
                .arg(downloaded)
                .output()
        }
        .map_err(|e| format!("Gagal menjalankan installer paket OS: {e}"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if err.is_empty() {
                format!(
                    "Instalasi paket dibatalkan atau gagal. Paket tersimpan di: {}",
                    downloaded.display()
                )
            } else {
                format!("Instalasi paket gagal: {err}")
            });
        }
        let _ = fs::remove_dir_all(stage_dir);
        let _ = Command::new("forge-desktop")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        return Ok(UpdateApplyResult {
            launched: true,
            restart_pending: true,
            message: "Paket sistem Linux berhasil diperbarui. Memulai ulang aplikasi...".into(),
            downloaded_path: None,
        });
    }

    let _ = Command::new("xdg-open").arg(downloaded).spawn();
    Ok(UpdateApplyResult {
        launched: true,
        restart_pending: false,
        message: format!(
            "Paket pembaruan terunduh & terverifikasi di {}. Silakan selesaikan instalasi lewat package manager OS.",
            downloaded.display()
        ),
        downloaded_path: Some(downloaded.display().to_string()),
    })
}

#[cfg(target_os = "macos")]
fn find_app_bundle_recursive(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("app"))
                .unwrap_or(false)
            {
                return Some(path);
            }
            if let Some(found) = find_app_bundle_recursive(&path) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn current_macos_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let contents = exe.parent()?.parent()?;
    let app_dir = contents.parent()?;
    if app_dir
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("app"))
        .unwrap_or(false)
    {
        Some(app_dir.to_path_buf())
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn apply_macos_update(
    downloaded: &Path,
    stage_dir: &Path,
    asset_name: &str,
) -> Result<UpdateApplyResult, String> {
    let lower = asset_name.to_ascii_lowercase();
    if lower.ends_with(".dmg") {
        let _ = fs::remove_file(stage_dir.join(".keep"));
        Command::new("open")
            .arg(downloaded)
            .spawn()
            .map_err(|e| format!("Gagal membuka image DMG pembaruan: {e}"))?;
        return Ok(UpdateApplyResult {
            launched: true,
            restart_pending: false,
            message: format!(
                "DMG pembaruan terverifikasi dan dibuka ({}). Tarik aplikasi ke folder Applications untuk memperbarui.",
                downloaded.display()
            ),
            downloaded_path: Some(downloaded.display().to_string()),
        });
    }

    let extract_dir = stage_dir.join("extracted");
    fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Gagal membuat folder ekstraksi macOS: {e}"))?;
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(downloaded)
        .arg("-C")
        .arg(&extract_dir)
        .status()
        .map_err(|e| format!("Gagal mengekstrak arsip .app.tar.gz: {e}"))?;
    if !status.success() {
        return Err("Ekstraksi arsip .app.tar.gz gagal.".into());
    }
    let new_app = find_app_bundle_recursive(&extract_dir)
        .ok_or_else(|| "Bundle .app tidak ditemukan dalam arsip pembaruan.".to_string())?;
    let target_app = current_macos_app_bundle()
        .ok_or_else(|| "Aplikasi saat ini tidak berjalan dari dalam bundle .app.".to_string())?;
    let parent_dir = target_app
        .parent()
        .ok_or_else(|| "Folder induk bundle .app tidak valid.".to_string())?;
    let pid = std::process::id();

    if ensure_dir_writable(parent_dir).is_ok() {
        let helper = stage_dir.join("apply-macos-update.sh");
        let script = r#"#!/bin/sh
set -eu
PID="$1"
NEW_APP="$2"
TARGET_APP="$3"
STAGE_DIR="$4"
i=0
while kill -0 "$PID" 2>/dev/null && [ "$i" -lt 80 ]; do
  sleep 0.25
  i=$((i + 1))
done
BAK="${TARGET_APP}.bak"
rm -rf "$BAK"
mv "$TARGET_APP" "$BAK"
if mv "$NEW_APP" "$TARGET_APP"; then
  rm -rf "$BAK"
else
  mv "$BAK" "$TARGET_APP" || true
  exit 1
fi
xattr -dr com.apple.quarantine "$TARGET_APP" 2>/dev/null || true
open -n "$TARGET_APP"
rm -rf "$STAGE_DIR"
"#;
        fs::write(&helper, script)
            .map_err(|e| format!("Gagal menulis skrip pembaruan macOS: {e}"))?;
        Command::new("sh")
            .arg(&helper)
            .arg(pid.to_string())
            .arg(&new_app)
            .arg(&target_app)
            .arg(stage_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Gagal menjalankan proses pembaruan macOS: {e}"))?;

        return Ok(UpdateApplyResult {
            launched: true,
            restart_pending: true,
            message: "Bundle .app terverifikasi. Aplikasi akan diperbarui dan dimulai ulang otomatis.".into(),
            downloaded_path: Some(downloaded.display().to_string()),
        });
    }

    // /Applications owned by root: ask macOS Authorization dialog via osascript
    let escape_sh = |p: &Path| -> String {
        format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"))
    };
    let shell_cmd = format!(
        "rm -rf {bak} && mv {tgt} {bak} && mv {src} {tgt} && rm -rf {bak} && xattr -dr com.apple.quarantine {tgt} || true",
        bak = escape_sh(&PathBuf::from(format!("{}.bak", target_app.display()))),
        tgt = escape_sh(&target_app),
        src = escape_sh(&new_app),
    );
    let osa = format!(
        "do shell script \"{}\" with administrator privileges",
        shell_cmd.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let out = Command::new("osascript")
        .args(["-e", &osa])
        .output()
        .map_err(|e| format!("Gagal meminta otorisasi administrator macOS: {e}"))?;
    if !out.status.success() {
        return Err("Pembaruan ke /Applications dibatalkan atau otorisasi administrator ditolak.".into());
    }
    let _ = fs::remove_dir_all(stage_dir);
    let _ = Command::new("open").args(["-n"]).arg(&target_app).spawn();

    Ok(UpdateApplyResult {
        launched: true,
        restart_pending: true,
        message: "Aplikasi di /Applications berhasil diperbarui. Memulai ulang...".into(),
        downloaded_path: Some(target_app.display().to_string()),
    })
}

pub fn download_and_install_update(
    channel: UpdateChannel,
    token: Option<&str>,
) -> Result<UpdateApplyResult, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let package_kind = detect_package_kind();
    let ResolvedCandidate {
        release: release_opt,
        asset: asset_opt,
        sha256: expected_sha_opt,
        ..
    } = resolve_release_candidate(channel, token, package_kind)?;

    let release = release_opt.ok_or_else(|| "Belum ada rilis tersedia di GitHub Releases.".to_string())?;
    let latest_clean = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    if !is_newer_version(&latest_clean, &current) {
        return Err(format!("Aplikasi sudah memakai versi terbaru (v{current})."));
    }

    let asset = asset_opt.ok_or_else(|| {
        format!(
            "Artefak rilis untuk OS/mode '{}' ({}) belum ditemukan pada rilis v{}.",
            package_kind.as_str(),
            current_arch(),
            latest_clean
        )
    })?;

    let safe_name = Path::new(&asset.name)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty() && !s.contains(".."))
        .ok_or_else(|| "Nama file artefak pembaruan tidak valid.".to_string())?;

    let stage_dir = std::env::temp_dir().join(format!(
        "forge-desktop-update-{}-{}",
        latest_clean.replace(['/', '\\', ':'], "_"),
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&stage_dir);
    fs::create_dir_all(&stage_dir)
        .map_err(|e| format!("Gagal menyiapkan folder staging pembaruan: {e}"))?;

    let download_path = stage_dir.join(safe_name);
    download_asset_to_file(&asset, &download_path, token)?;

    let expected_sha = expected_sha_opt.ok_or_else(|| {
        let _ = fs::remove_dir_all(&stage_dir);
        "Checksum SHA-256 artefak tidak tersedia pada rilis (SHA256SUMS/digest kosong). Pembaruan dibatalkan demi keamanan.".to_string()
    })?;
    if let Err(err) = verify_file_sha256(&download_path, &expected_sha) {
        let _ = fs::remove_dir_all(&stage_dir);
        return Err(err);
    }

    #[cfg(target_os = "windows")]
    {
        apply_windows_update(package_kind, &download_path, &stage_dir, safe_name)
    }
    #[cfg(target_os = "linux")]
    {
        apply_linux_update(package_kind, &download_path, &stage_dir, safe_name)
    }
    #[cfg(target_os = "macos")]
    {
        apply_macos_update(&download_path, &stage_dir, safe_name)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = fs::remove_dir_all(&stage_dir);
        Err("OS saat ini tidak mendukung pembaruan otomatis.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_asset(name: &str) -> GhReleaseAsset {
        GhReleaseAsset {
            name: name.to_string(),
            url: None,
            browser_download_url: format!("https://github.com/machfudn/forge-desktop/releases/download/v1.2.0/{name}"),
            size: Some(1024),
            digest: Some("sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()),
        }
    }

    #[test]
    fn semver_ordering_handles_stable_and_beta() {
        assert!(is_newer_version("1.2.0", "1.1.0"));
        assert!(is_newer_version("1.2.0-beta.1", "1.1.0"));
        assert!(is_newer_version("1.2.0-beta.2", "1.2.0-beta.1"));
        assert!(is_newer_version("1.2.0-beta.10", "1.2.0-beta.2"));
        assert!(is_newer_version("1.2.0-rc.1", "1.2.0-beta.9"));
        assert!(is_newer_version("1.2.0", "1.2.0-beta.5"));
        assert!(!is_newer_version("1.2.0-beta.5", "1.2.0"));
        assert!(!is_newer_version("1.2.0", "1.2.0"));
    }

    #[test]
    fn channel_pick_filters_prereleases_for_stable() {
        let releases = vec![
            GhRelease {
                tag_name: "v1.1.0".into(),
                body: None,
                html_url: None,
                draft: false,
                prerelease: false,
                assets: vec![],
            },
            GhRelease {
                tag_name: "v1.2.0-beta.2".into(),
                body: None,
                html_url: None,
                draft: false,
                prerelease: true,
                assets: vec![],
            },
            GhRelease {
                tag_name: "v1.3.0-alpha.1".into(),
                body: None,
                html_url: None,
                draft: true,
                prerelease: true,
                assets: vec![],
            },
        ];

        assert_eq!(
            pick_target_release(&releases, "stable").map(|r| r.tag_name.as_str()),
            Some("v1.1.0")
        );
        assert_eq!(
            pick_target_release(&releases, "beta").map(|r| r.tag_name.as_str()),
            Some("v1.2.0-beta.2")
        );
    }

    #[test]
    fn asset_selection_matches_portable_and_installed_modes() {
        let assets = vec![
            mk_asset("Forge.Desktop_1.2.0_x64-setup.exe"),
            mk_asset("Forge.Desktop_1.2.0_x64_en-US.msi"),
            mk_asset("Forge-Desktop_1.2.0_windows_x64-portable.exe"),
            mk_asset("Forge-Desktop_1.2.0_windows_x64-portable.zip"),
            mk_asset("Forge.Desktop_1.2.0_amd64.deb"),
            mk_asset("Forge.Desktop-1.2.0-1.x86_64.rpm"),
            mk_asset("Forge.Desktop_1.2.0_amd64.AppImage"),
            mk_asset("Forge-Desktop_1.2.0_linux_x64-portable.tar.gz"),
            mk_asset("Forge.Desktop_1.2.0_universal.dmg"),
            mk_asset("Forge.Desktop_1.2.0_universal.app.tar.gz"),
            mk_asset("SHA256SUMS"),
            mk_asset("SHA256SUMS.asc"),
        ];

        assert_eq!(
            select_best_asset(&assets, "x86_64", PackageKind::WindowsPortable).map(|a| a.name.as_str()),
            Some("Forge-Desktop_1.2.0_windows_x64-portable.exe")
        );
        assert_eq!(
            select_best_asset(&assets, "x86_64", PackageKind::WindowsNsis).map(|a| a.name.as_str()),
            Some("Forge.Desktop_1.2.0_x64-setup.exe")
        );
        assert_eq!(
            select_best_asset(&assets, "x86_64", PackageKind::WindowsMsi).map(|a| a.name.as_str()),
            Some("Forge.Desktop_1.2.0_x64_en-US.msi")
        );
        assert_eq!(
            select_best_asset(&assets, "x86_64", PackageKind::LinuxAppImage).map(|a| a.name.as_str()),
            Some("Forge.Desktop_1.2.0_amd64.AppImage")
        );
        assert_eq!(
            select_best_asset(&assets, "x86_64", PackageKind::LinuxPortable).map(|a| a.name.as_str()),
            Some("Forge-Desktop_1.2.0_linux_x64-portable.tar.gz")
        );
        assert_eq!(
            select_best_asset(&assets, "x86_64", PackageKind::LinuxDeb).map(|a| a.name.as_str()),
            Some("Forge.Desktop_1.2.0_amd64.deb")
        );
        assert_eq!(
            select_best_asset(&assets, "x86_64", PackageKind::LinuxRpm).map(|a| a.name.as_str()),
            Some("Forge.Desktop-1.2.0-1.x86_64.rpm")
        );
        assert_eq!(
            select_best_asset(&assets, "aarch64", PackageKind::MacOsInstalled).map(|a| a.name.as_str()),
            Some("Forge.Desktop_1.2.0_universal.app.tar.gz")
        );
    }

    #[test]
    fn parse_sha256sums_and_verify_file() {
        let tmp = std::env::temp_dir().join(format!("forge-sha-test-{}.bin", std::process::id()));
        fs::write(&tmp, b"forge-desktop-update-test").unwrap();
        let digest = compute_file_sha256(&tmp).unwrap();
        let text = format!("{digest}  ./Forge-Desktop_1.2.0_windows_x64-portable.exe\n");
        let parsed = parse_sha256sums(&text);
        assert_eq!(
            parsed.get("forge-desktop_1.2.0_windows_x64-portable.exe").map(String::as_str),
            Some(digest.as_str())
        );
        assert!(verify_file_sha256(&tmp, &digest).is_ok());
        assert!(verify_file_sha256(&tmp, &"0".repeat(64)).is_err());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn detect_windows_portable_vs_installed() {
        assert_eq!(
            detect_package_kind_for(
                "windows",
                Path::new("D:\\Apps\\Forge-Desktop-portable.exe"),
                false
            ),
            PackageKind::WindowsPortable
        );
        assert_eq!(
            detect_package_kind_for(
                "windows",
                Path::new("C:\\Program Files\\Forge Desktop\\forge-desktop.exe"),
                false
            ),
            PackageKind::WindowsMsi
        );
        assert_eq!(
            detect_package_kind_for(
                "windows",
                Path::new("C:\\Users\\Demo\\AppData\\Local\\Forge Desktop\\forge-desktop.exe"),
                false
            ),
            PackageKind::WindowsNsis
        );
        assert_eq!(
            detect_package_kind_for("linux", Path::new("/tmp/.mount_abc/usr/bin/forge"), true),
            PackageKind::LinuxAppImage
        );
        assert_eq!(
            detect_package_kind_for("macos", Path::new("/Applications/Forge Desktop.app/Contents/MacOS/forge"), false),
            PackageKind::MacOsInstalled
        );
        assert_eq!(
            detect_package_kind_for("macos", Path::new("/Users/me/Downloads/Forge Desktop.app/Contents/MacOS/forge"), false),
            PackageKind::MacOsApp
        );
    }

    #[test]
    fn trusted_github_urls_reject_http_and_non_github() {
        assert!(is_trusted_github_url("https://github.com/machfudn/forge-desktop/releases/download/v1.0.0/a.exe"));
        assert!(is_trusted_github_url("https://objects.githubusercontent.com/github-production-release-asset/123"));
        assert!(!is_trusted_github_url("http://github.com/a/b"));
        assert!(!is_trusted_github_url("https://evil.example.com/a.exe"));
    }
}
