use crate::model::{AppEntry, Arch, RegRoot, Source};
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

pub struct ScanResult {
    pub apps: Vec<AppEntry>,
    pub warnings: Vec<String>,
}

fn get_str(key: &RegKey, name: &str) -> Option<String> {
    match key.get_value::<String, _>(name) {
        Ok(s) => {
            let t = s.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        }
        Err(_) => None,
    }
}

fn get_u32(key: &RegKey, name: &str) -> Option<u32> {
    key.get_value::<u32, _>(name).ok()
}

/// Split a command line into (executable, arguments), honouring quotes.
pub fn split_command(cmd: &str) -> (String, String) {
    let s = cmd.trim();
    if s.is_empty() {
        return (String::new(), String::new());
    }
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            let exe = rest[..end].to_string();
            let args = rest[end + 1..].trim().to_string();
            return (exe, args);
        }
    }
    // Unquoted: split before the first " /" or " -" that looks like a switch.
    let lower = s.to_lowercase();
    for pat in [" /", " -", "\t/"] {
        if let Some(idx) = lower.find(pat) {
            if idx > 3 {
                return (s[..idx].trim().to_string(), s[idx..].trim().to_string());
            }
        }
    }
    (s.to_string(), String::new())
}

fn msi_product_from(cmd: &str) -> Option<String> {
    let lower = cmd.to_lowercase();
    if !lower.contains("msiexec") {
        return None;
    }
    let bytes: Vec<char> = cmd.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '{' {
            if let Some(end) = bytes[i..].iter().position(|&c| c == '}') {
                let guid: String = bytes[i..i + end + 1].iter().collect();
                if guid.len() == 38 {
                    return Some(guid);
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_date(raw: &str) -> String {
    let d: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() >= 8 {
        format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8])
    } else {
        String::new()
    }
}

fn normalize(p: &str) -> String {
    p.trim().trim_matches('"').replace('/', "\\").to_lowercase()
}

/// Windows resolves `.` and `..` itself, so a lexical prefix would claim a
/// target it does not own. Refuse those targets instead of guessing.
fn has_dot_segments(p: &str) -> bool {
    p.split(['\\', '/']).any(|c| c == "." || c == "..")
}

fn is_under(path: &str, root: &str) -> bool {
    if has_dot_segments(path) || has_dot_segments(root) {
        return false;
    }
    let p = normalize(path);
    let r = normalize(root);
    // A component boundary is required: `D:\SharedToolsExtra` is not inside
    // `D:\SharedTools`, and the drive root still matches everything on it.
    let r = r.trim_end_matches('\\');
    if r.is_empty() {
        return false;
    }
    p == r || (p.starts_with(r) && p.as_bytes().get(r.len()) == Some(&b'\\'))
}

/// True when a start-menu shortcut target belongs to `app`: the same
/// executable, or any executable inside the folder of an app whose exe is not
/// known yet. A different known exe in that folder is another application.
fn shortcut_target_belongs(app: &AppEntry, target: &str) -> bool {
    match &app.launch_exe {
        Some(exe) => normalize(&exe.to_string_lossy()) == normalize(target),
        None => app
            .install_dir
            .as_ref()
            .map(|d| is_under(target, &d.to_string_lossy()))
            .unwrap_or(false),
    }
}

/// Words too generic to identify a product on their own.
fn weak_word(w: &str) -> bool {
    matches!(
        w,
        "application"
            | "applications"
            | "program"
            | "programs"
            | "app"
            | "apps"
            | "bin"
            | "x86"
            | "x64"
            | "arm64"
            | "version"
            | "edition"
            | "setup"
            | "installer"
            | "install"
            | "launcher"
            | "update"
            | "updater"
            | "helper"
            | "service"
            | "client"
            | "server"
            | "manager"
            | "tool"
            | "tools"
            | "runtime"
            | "package"
            | "external"
            | "microsoft"
            | "windows"
            | "google"
            | "corp"
            | "corporation"
            | "inc"
            | "ltd"
            | "llc"
            | "the"
            | "for"
            | "and"
            | "with"
            | "bit"
            | "media"
            | "player"
            | "browser"
            | "data"
            | "sources"
            | "source"
            | "connection"
            | "remote"
            | "desktop"
            | "suite"
            | "studio"
            | "control"
            | "panel"
            | "panels"
            | "audio"
            | "video"
            | "lite"
            | "free"
            | "plus"
            | "core"
            | "sync"
            | "drive"
            | "reader"
            | "viewer"
            | "editor"
            | "note"
            | "notes"
            | "shell"
            | "dev"
            | "pro"
            | "pdf"
            | "disk"
            | "digital"
            | "online"
            | "cloud"
            | "security"
            | "system"
            | "systems"
            | "network"
            | "web"
            | "mobile"
            | "point"
            | "home"
            | "personal"
            | "standard"
            | "basic"
            | "premium"
            | "ultimate"
            | "professional"
            | "express"
    )
}

fn name_words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_lowercase())
        .filter(|w| !weak_word(w))
        .collect()
}

/// Strict relation test used by the dedupe pass: every distinctive word of the
/// shorter name must appear in the longer one.
fn names_related(a: &str, b: &str) -> bool {
    let wa = name_words(a);
    let wb = name_words(b);
    if wa.is_empty() || wb.is_empty() {
        return false;
    }
    let (small, big) = if wa.len() <= wb.len() {
        (&wa, &wb)
    } else {
        (&wb, &wa)
    };
    small.iter().all(|w| big.iter().any(|x| x == w))
}

/// Picks the single best registry entry that a shortcut name belongs to.
/// A candidate only qualifies when *every* distinctive word of the shorter name
/// appears in the longer one, so a shared generic word such as "player" or
/// "media" can never attach the wrong folder. Returns `None` when nothing
/// qualifies or when two candidates tie, because a wrong path is far worse than
/// a duplicate row.
fn best_name_match<'a, I>(stem: &str, candidates: I) -> Option<usize>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mine = name_words(stem);
    if mine.is_empty() {
        return None;
    }
    let mut scored: Vec<(usize, usize)> = Vec::new(); // (idx, specificity)
    for (idx, name) in candidates {
        let theirs = name_words(name);
        if theirs.is_empty() {
            continue;
        }
        let (small, big) = if mine.len() <= theirs.len() {
            (&mine, &theirs)
        } else {
            (&theirs, &mine)
        };
        if small.iter().all(|w| big.iter().any(|x| x == w)) {
            scored.push((idx, big.len()));
        }
    }
    scored.sort_by_key(|x| std::cmp::Reverse(x.1));
    match scored.as_slice() {
        [] => None,
        [one] => Some(one.0),
        [best, second, ..] => {
            if best.1 == second.1 {
                None
            } else {
                Some(best.0)
            }
        }
    }
}

/// A package whose display name is missing and whose identifier is a bare GUID
/// is an internal system component that Windows itself never shows.
fn looks_like_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let lens = [8usize, 4, 4, 4, 12];
    parts
        .iter()
        .zip(lens.iter())
        .all(|(p, l)| p.len() == *l && p.chars().all(|c| c.is_ascii_hexdigit()))
}

fn dir_exists(p: &Path) -> bool {
    p.is_dir()
}

/// Best-effort guess of the main executable inside an install folder.
fn guess_main_exe(dir: &Path, app_name: &str) -> Option<PathBuf> {
    let mut best: Option<(u32, PathBuf)> = None;
    let needle: String = app_name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten().take(400) {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "exe" {
            continue;
        }
        let stem: String = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let mut score = 0u32;
        if stem.contains(&needle) && !needle.is_empty() {
            score += 60;
        }
        if !needle.is_empty() && needle.contains(&stem) && stem.len() > 3 {
            score += 30;
        }
        for bad in [
            "unins",
            "setup",
            "update",
            "crash",
            "helper",
            "report",
            "vc_redist",
        ] {
            if stem.contains(bad) {
                score = score.saturating_sub(80);
            }
        }
        if stem == "launcher" || stem == "start" {
            score += 5;
        }
        if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, p));
        }
    }
    best.filter(|(s, _)| *s > 0).map(|(_, p)| p)
}

/// Reads the PE machine field to determine the real architecture of an exe.
/// Returns `None` when the file is unreadable or not a PE image.
fn pe_arch(p: &Path) -> Option<Arch> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).ok()?;
    let mut mz = [0u8; 2];
    f.read_exact(&mut mz).ok()?;
    if &mz != b"MZ" {
        return None;
    }
    f.seek(SeekFrom::Start(0x3c)).ok()?;
    let mut off = [0u8; 4];
    f.read_exact(&mut off).ok()?;
    let pe = u32::from_le_bytes(off) as u64;
    if pe == 0 || pe > 0x1000_0000 {
        return None;
    }
    f.seek(SeekFrom::Start(pe)).ok()?;
    let mut sig = [0u8; 6];
    f.read_exact(&mut sig).ok()?;
    if &sig[..4] != b"PE\0\0" {
        return None;
    }
    match u16::from_le_bytes([sig[4], sig[5]]) {
        0x014c => Some(Arch::X86),
        0x8664 => Some(Arch::X64),
        0xaa64 => Some(Arch::Arm64),
        0x01c4 => Some(Arch::Arm32),
        _ => Some(Arch::Neutral),
    }
}

fn is_system_path(p: &Path) -> bool {
    let s = p.to_string_lossy().to_lowercase();
    if s.contains("\\windowsapps\\") {
        return false;
    }
    let win = std::env::var("SystemRoot")
        .unwrap_or_else(|_| "C:\\Windows".to_string())
        .to_lowercase();
    s.starts_with(&win) && !s.starts_with(&format!("{}\\installer", win))
}

fn push_unique(list: &mut Vec<PathBuf>, p: PathBuf) {
    if p.as_os_str().is_empty() {
        return;
    }
    let n = normalize(&p.to_string_lossy());
    if !list.iter().any(|x| normalize(&x.to_string_lossy()) == n) {
        list.push(p);
    }
}

// ---------------------------------------------------------------- registry

fn scan_registry(root: RegRoot, path: &str, apps: &mut Vec<AppEntry>, next_id: &mut u64) {
    let (hive, view) = match root {
        RegRoot::Hklm => (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
        RegRoot::Hklm32 => (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
        RegRoot::Hkcu => (HKEY_CURRENT_USER, KEY_READ),
    };
    let base = RegKey::predef(hive);
    let key = match base.open_subkey_with_flags(path, view) {
        Ok(k) => k,
        Err(_) => return,
    };

    for sub in key.enum_keys().flatten() {
        let sk = match key.open_subkey_with_flags(&sub, view) {
            Ok(k) => k,
            Err(_) => continue,
        };

        // Hidden from the Programs list by Windows itself, but still an
        // installed app, so it stays in the list and gets classified below.
        let system_component = get_u32(&sk, "SystemComponent").unwrap_or(0) == 1;
        let name = match get_str(&sk, "DisplayName") {
            Some(n) => n,
            None => continue,
        };
        if get_u32(&sk, "NoDisplay").unwrap_or(0) == 1 {
            continue;
        }
        // Internal components that Windows hides from the Programs list.
        if looks_like_guid(name.trim()) {
            continue;
        }

        let uninstall = get_str(&sk, "UninstallString");
        let quiet = get_str(&sk, "QuietUninstallString");
        let display_icon = get_str(&sk, "DisplayIcon");
        let install_loc = get_str(&sk, "InstallLocation");
        let publisher = get_str(&sk, "Publisher").unwrap_or_default();
        let version = get_str(&sk, "DisplayVersion").unwrap_or_default();
        let size_kb = get_u32(&sk, "EstimatedSize").unwrap_or(0) as u64;

        let mut install_dir: Option<PathBuf> = None;
        if let Some(loc) = &install_loc {
            let p = PathBuf::from(loc.trim().trim_matches('"'));
            if dir_exists(&p) {
                install_dir = Some(p);
            }
        }

        let mut launch_exe: Option<PathBuf> = None;
        if let Some(icon) = &display_icon {
            let (file, _) = split_command(icon);
            let p = PathBuf::from(file.trim().trim_matches('"'));
            if p.is_file() {
                let ext = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "exe" {
                    launch_exe = Some(p.clone());
                }
                if install_dir.is_none() {
                    if let Some(parent) = p.parent() {
                        install_dir = Some(parent.to_path_buf());
                    }
                }
            }
        }
        if install_dir.is_none() {
            if let Some(u) = &uninstall {
                let (file, _) = split_command(u);
                let p = PathBuf::from(file.trim());
                if let Some(parent) = p.parent() {
                    let s = parent.to_string_lossy().to_lowercase();
                    if s.contains("program files") && dir_exists(parent) {
                        install_dir = Some(parent.to_path_buf());
                    }
                }
            }
        }
        if launch_exe.is_none() {
            if let Some(d) = &install_dir {
                launch_exe = guess_main_exe(d, &name);
            }
        }
        // A missing install folder is common for MSI products (the registry
        // records no InstallLocation). Use the uninstaller's own executable so
        // the row still has a real icon and a reveal target.
        let mut uninstaller_exe: Option<PathBuf> = None;
        if let Some(u) = &uninstall {
            let (file, _) = split_command(u);
            let p = PathBuf::from(file.trim().trim_matches('"'));
            if p.is_file() {
                uninstaller_exe = Some(p);
            }
        }

        let msi = uninstall.as_deref().and_then(msi_product_from);
        let mut arch = Arch::Neutral;
        if root == RegRoot::Hklm32 {
            arch = Arch::X86;
        } else if root == RegRoot::Hklm {
            arch = Arch::X64;
        }
        if let Some(u) = &uninstall {
            let l = u.to_lowercase();
            if l.contains("syswow64") {
                arch = Arch::X86;
            }
        }
        // The registry view only says which hive the entry lives in, so trust
        // the real executable when we have one. This is also the only way to
        // spot ARM64 builds.
        if let Some(a) = launch_exe.as_deref().and_then(pe_arch) {
            arch = a;
        } else if let Some(d) = &install_dir {
            let l = d.to_string_lossy().to_lowercase();
            if l.contains("arm64") || l.contains("aarch64") {
                arch = Arch::Arm64;
            }
        }

        let is_system =
            install_dir.as_deref().map(is_system_path).unwrap_or(false) || system_component;

        let mut e = AppEntry::new(*next_id, name, Source::Registry);
        *next_id += 1;
        e.publisher = publisher;
        e.version = version;
        e.install_dir = install_dir;
        e.launch_exe = launch_exe.clone();
        e.icon_path = launch_exe.or(uninstaller_exe);
        e.uninstall_string = uninstall;
        e.quiet_uninstall_string = quiet;
        e.msi_product = msi;
        e.install_date = get_str(&sk, "InstallDate")
            .map(|d| parse_date(&d))
            .unwrap_or_default();
        e.size_bytes = size_kb * 1024;
        e.arch = arch;
        e.is_system = is_system;
        e.reg_key = Some((root, format!("{}\\{}", path, sub)));
        apps.push(e);
    }
}

// ---------------------------------------------------------------- UWP

fn scan_uwp(apps: &mut Vec<AppEntry>, next_id: &mut u64, warnings: &mut Vec<String>) {
    use windows::core::HSTRING;
    use windows::Management::Deployment::PackageManager;

    let sid = crate::native::current_user_sid();
    if sid.is_empty() {
        warnings.push("Could not read the user ID".into());
        return;
    }
    let pm = match PackageManager::new() {
        Ok(p) => p,
        Err(e) => {
            warnings.push(format!("Could not reach the package manager: {e}"));
            return;
        }
    };
    let list = match pm.FindPackagesByUserSecurityId(&HSTRING::from(sid.as_str())) {
        Ok(l) => l,
        Err(e) => {
            warnings.push(format!("Could not enumerate Store apps: {e}"));
            return;
        }
    };

    for pkg in list {
        let id = match pkg.Id() {
            Ok(i) => i,
            Err(_) => continue,
        };
        let full = id.FullName().map(|h| h.to_string()).unwrap_or_default();
        let family = id.FamilyName().map(|h| h.to_string()).unwrap_or_default();
        let mut name = pkg.DisplayName().map(|h| h.to_string()).unwrap_or_default();
        if name.is_empty() || name.starts_with("ms-resource:") {
            name = id
                .Name()
                .map(|h| h.to_string())
                .unwrap_or_else(|_| family.clone());
        }
        if name.is_empty() {
            continue;
        }
        // A bare GUID means Windows exposes no display name for this package.
        // Keep it listed (the user asked for every app) but label it clearly.
        let unnamed = looks_like_guid(name.trim());
        if unnamed {
            name = "Unnamed system package".to_string();
        }
        let publisher = pkg
            .PublisherDisplayName()
            .map(|h| h.to_string())
            .unwrap_or_default();
        let install_path = pkg
            .InstalledPath()
            .map(|h| h.to_string())
            .unwrap_or_default();
        let is_framework = pkg.IsFramework().unwrap_or(false);

        let mut e = AppEntry::new(*next_id, name, Source::Uwp);
        *next_id += 1;
        e.publisher = publisher;
        e.version = id
            .Version()
            .map(|v| format!("{}.{}.{}.{}", v.Major, v.Minor, v.Build, v.Revision))
            .unwrap_or_default();
        e.package_full_name = Some(full.clone());
        e.is_framework = is_framework;
        // Store package names carry the architecture, e.g.
        // `MSTeams_26260.1717_x64__8wekyb3d8bbwe`.
        e.arch = if full.contains("_x64__") {
            Arch::X64
        } else if full.contains("_x86__") {
            Arch::X86
        } else if full.contains("_arm64__") {
            Arch::Arm64
        } else {
            Arch::Neutral
        };

        if !install_path.is_empty() {
            let dir = PathBuf::from(&install_path);
            e.install_dir = Some(dir.clone());
            // Resolve the logo image, preferring a 44x44 or 48x48 square asset.
            if let Ok(uri) = pkg.Logo() {
                let abs = uri.AbsoluteUri().map(|h| h.to_string()).unwrap_or_default();
                if let Some(rel) = ms_appx_relative(&abs) {
                    let candidate = dir.join(rel);
                    if candidate.is_file() {
                        e.icon_path = Some(candidate);
                    }
                }
            }
            // The expensive directory walk happens later, in parallel.
        }
        e.is_system = is_framework || e.publisher.to_lowercase().contains("microsoft");
        apps.push(e);
    }
}

fn ms_appx_relative(uri: &str) -> Option<String> {
    let u = uri.trim();
    if let Some(rest) = u.strip_prefix("ms-appx://") {
        // Skip the package name segment.
        let idx = rest.find('/')?;
        return Some(rest[idx + 1..].replace('/', "\\"));
    }
    if u.starts_with("ms-resource:") {
        return None;
    }
    if u.len() > 3 && u.as_bytes()[1] == b':' {
        return Some(u.to_string());
    }
    None
}

fn find_package_logo(dir: &Path) -> Option<PathBuf> {
    fn walk(dir: &Path, depth: usize, best: &mut Option<(u32, PathBuf)>) {
        if depth > 2 {
            return;
        }
        let rd = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        for e in rd.flatten().take(600) {
            let p = e.path();
            if p.is_dir() {
                walk(&p, depth + 1, best);
                continue;
            }
            let fname = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !fname.ends_with(".png") {
                continue;
            }
            let mut score = 0u32;
            if fname.contains("square44x44logo") {
                score += 100;
            } else if fname.contains("square48x48") || fname.contains("48x48") {
                score += 90;
            } else if fname.contains("square150x150") {
                score += 60;
            } else if fname.contains("logo") {
                score += 20;
            }
            if fname.contains("targetsize") {
                score += 15;
            }
            if fname.contains("scale-100") {
                score += 8;
            }
            if score == 0 {
                continue;
            }
            if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                *best = Some((score, p.clone()));
            }
        }
    }
    let mut best = None;
    walk(dir, 0, &mut best);
    best.map(|(_, p)| p)
}

// ---------------------------------------------------------------- shortcuts

fn start_menu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(pd) = std::env::var("ProgramData") {
        dirs.push(PathBuf::from(pd).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Ok(ad) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(ad).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Ok(up) = std::env::var("USERPROFILE") {
        dirs.push(PathBuf::from(&up).join("Desktop"));
        dirs.push(PathBuf::from(up).join("OneDrive\\Desktop"));
    }
    if let Ok(pd) = std::env::var("PUBLIC") {
        dirs.push(PathBuf::from(pd).join("Desktop"));
    }
    dirs.into_iter().filter(|d| d.is_dir()).collect()
}

fn collect_lnk(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 {
        return;
    }
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_lnk(&p, out, depth + 1);
        } else if p
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("lnk"))
            .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

fn scan_shortcuts(apps: &mut Vec<AppEntry>, next_id: &mut u64) {
    let mut links = Vec::new();
    for d in start_menu_dirs() {
        collect_lnk(&d, &mut links, 0);
    }

    for lnk in links {
        let (target, _args, _wd) = match crate::native::resolve_shortcut(&lnk.to_string_lossy()) {
            Some(t) => t,
            None => continue,
        };
        if target.is_empty() {
            continue;
        }
        let tl = target.to_lowercase();
        if !tl.ends_with(".exe") {
            continue;
        }
        if tl.contains("\\windowsapps\\") {
            // Belongs to a store package which we already list.
            continue;
        }
        let stem = lnk
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if stem.is_empty() {
            continue;
        }
        // Attach to an existing entry when the shortcut points inside its folder.
        // Otherwise, try to reunite it with a registry product that has no
        // install path (typical for MSI installers).
        let exe = PathBuf::from(&target);
        let mut attached = false;
        for app in apps.iter_mut() {
            // Exact executable match, or an app that knows its folder but not
            // its exe may adopt a target inside it. A different exe in the same
            // folder is a different application, not this one's shortcut.
            let hit = shortcut_target_belongs(app, &target);
            if hit {
                push_unique(&mut app.shortcuts, lnk.clone());
                if app.launch_exe.is_none() {
                    app.launch_exe = Some(PathBuf::from(&target));
                }
                if app.icon_path.is_none() {
                    app.icon_path = Some(PathBuf::from(&target));
                }
                if app.install_dir.is_none() {
                    if let Some(parent) = exe.parent() {
                        app.install_dir = Some(parent.to_path_buf());
                    }
                }
                attached = true;
                break;
            }
        }

        if !attached {
            let candidates = apps.iter().enumerate().filter_map(|(i, a)| {
                let pathless = a.install_dir.is_none() && a.launch_exe.is_none();
                if pathless && a.source == Source::Registry {
                    Some((i, a.name.as_str()))
                } else {
                    None
                }
            });
            if let Some(idx) = best_name_match(&stem, candidates) {
                let app = &mut apps[idx];
                push_unique(&mut app.shortcuts, lnk.clone());
                app.launch_exe = Some(exe.clone());
                if app.icon_path.is_none() {
                    app.icon_path = Some(exe.clone());
                }
                if let Some(parent) = exe.parent() {
                    app.install_dir = Some(parent.to_path_buf());
                }
                attached = true;
            }
        }
        if attached {
            continue;
        }
        let dir = exe.parent().map(|p| p.to_path_buf());
        let mut e = AppEntry::new(*next_id, stem, Source::Shortcut);
        *next_id += 1;
        e.launch_exe = Some(exe);
        e.icon_path = e.launch_exe.clone();
        e.install_dir = dir;
        e.shortcuts.push(lnk);
        e.is_system = e
            .install_dir
            .as_deref()
            .map(is_system_path)
            .unwrap_or(false);
        apps.push(e);
    }
}

// ---------------------------------------------------------------- dedupe + entry point

fn dedupe(apps: Vec<AppEntry>) -> Vec<AppEntry> {
    let mut out: Vec<AppEntry> = Vec::new();
    for app in apps {
        let mut merged = false;
        for existing in out.iter_mut() {
            let same_exe = match (&app.launch_exe, &existing.launch_exe) {
                (Some(a), Some(b)) => {
                    normalize(&a.to_string_lossy()) == normalize(&b.to_string_lossy())
                }
                _ => false,
            };
            let same_key = app.reg_key.is_some() && app.reg_key == existing.reg_key;
            let same_package = match (&app.package_full_name, &existing.package_full_name) {
                (Some(a), Some(b)) => !a.is_empty() && a.eq_ignore_ascii_case(b),
                _ => false,
            };
            // A shortcut-only entry whose name matches a registry product that
            // has no install path: same app, so fold them together.
            let name_bridge = app.source == Source::Shortcut
                && existing.source == Source::Registry
                && existing.install_dir.is_none()
                && app.install_dir.is_some()
                && names_related(&app.name, &existing.name);
            // Two registrations with different keys are two removable products,
            // even when they share a folder, a name or an executable. Keeping
            // them apart keeps every key in the backup, delete and verify plans.
            let keys_conflict =
                matches!((&app.reg_key, &existing.reg_key), (Some(a), Some(b)) if a != b);
            if (same_exe || same_key || same_package || name_bridge) && !keys_conflict {
                // Prefer registry data, but keep useful fields from both.
                if existing.uninstall_string.is_none() {
                    existing.uninstall_string = app.uninstall_string.clone();
                }
                if existing.quiet_uninstall_string.is_none() {
                    existing.quiet_uninstall_string = app.quiet_uninstall_string.clone();
                }
                if existing.install_dir.is_none() {
                    existing.install_dir = app.install_dir.clone();
                }
                if existing.launch_exe.is_none() {
                    existing.launch_exe = app.launch_exe.clone();
                }
                if existing.icon_path.is_none() {
                    existing.icon_path = app.icon_path.clone();
                }
                if existing.publisher.is_empty() {
                    existing.publisher = app.publisher.clone();
                }
                if existing.version.is_empty() {
                    existing.version = app.version.clone();
                }
                if existing.size_bytes == 0 {
                    existing.size_bytes = app.size_bytes;
                }
                if existing.install_date.is_empty() {
                    existing.install_date = app.install_date.clone();
                }
                if existing.reg_key.is_none() {
                    existing.reg_key = app.reg_key.clone();
                }
                if existing.package_full_name.is_none() {
                    existing.package_full_name = app.package_full_name.clone();
                }
                for s in &app.shortcuts {
                    push_unique(&mut existing.shortcuts, s.clone());
                }
                if existing.source == Source::Shortcut && app.source != Source::Shortcut {
                    existing.source = app.source;
                    existing.reg_key = app.reg_key.clone();
                    existing.package_full_name = app.package_full_name.clone();
                }
                merged = true;
                break;
            }
        }
        if !merged {
            out.push(app);
        }
    }
    out
}

/// Filesystem work that is too slow to block the first paint: package logos
/// and folder sizes.
pub struct Enrich {
    pub id: u64,
    pub icon: Option<PathBuf>,
    pub size: u64,
}

/// Resolve logos/sizes for `apps` on several threads, calling `progress` with
/// small batches as they finish.
pub fn enrich_apps(apps: &[AppEntry], progress: impl Fn(Vec<Enrich>) + Send + Sync + 'static) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let jobs: Vec<(u64, PathBuf, bool, bool)> = apps
        .iter()
        .filter_map(|a| {
            let dir = a.install_dir.clone()?;
            let need_logo = a.icon_path.is_none() && a.source == Source::Uwp;
            let need_size = a.size_bytes == 0;
            if !need_logo && !need_size {
                return None;
            }
            Some((a.id, dir, need_logo, need_size))
        })
        .collect();

    if jobs.is_empty() {
        return;
    }

    let jobs = Arc::new(jobs);
    let cursor = Arc::new(AtomicUsize::new(0));
    let progress = Arc::new(progress);
    let n = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(4)
        .clamp(1, 8);
    let mut handles = Vec::new();
    for _ in 0..n {
        let jobs = jobs.clone();
        let cursor = cursor.clone();
        let progress = progress.clone();
        handles.push(std::thread::spawn(move || {
            let mut batch = Vec::new();
            loop {
                let i = cursor.fetch_add(1, Ordering::Relaxed);
                if i >= jobs.len() {
                    break;
                }
                let (id, dir, need_logo, need_size) = &jobs[i];
                let logo = if *need_logo {
                    find_package_logo(dir)
                } else {
                    None
                };
                let size = if *need_size {
                    crate::native::path_size(&dir.to_string_lossy())
                } else {
                    0
                };
                batch.push(Enrich {
                    id: *id,
                    icon: logo,
                    size,
                });
                if batch.len() >= 8 {
                    progress(std::mem::take(&mut batch));
                }
            }
            if !batch.is_empty() {
                progress(batch);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
}

pub fn scan_all() -> ScanResult {
    let mut warnings = Vec::new();
    let mut apps: Vec<AppEntry> = Vec::new();
    let mut next_id: u64 = 1;

    for (root, path) in [
        (
            RegRoot::Hklm,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            RegRoot::Hklm32,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            RegRoot::Hkcu,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
    ] {
        scan_registry(root, path, &mut apps, &mut next_id);
    }

    scan_uwp(&mut apps, &mut next_id, &mut warnings);
    scan_shortcuts(&mut apps, &mut next_id);

    let mut apps = dedupe(apps);

    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.publisher.cmp(&b.publisher))
    });

    ScanResult { apps, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_words_are_generic_only() {
        for w in ["application", "windows", "microsoft", "x64", "launcher"] {
            assert!(weak_word(w), "{w} should be weak");
        }
        for w in ["roblox", "chrome", "krita", "verifier"] {
            assert!(!weak_word(w), "{w} should be distinctive");
        }
    }

    #[test]
    fn name_words_drops_generic_parts() {
        assert_eq!(name_words("Google Chrome"), vec!["chrome".to_string()]);
        assert_eq!(name_words("Roblox Player"), vec!["roblox".to_string()]);
        assert!(name_words("Windows Media Player").is_empty());
    }

    #[test]
    fn names_related_needs_shared_distinctive_words() {
        assert!(names_related("Roblox", "Roblox Player"));
        assert!(!names_related("Roblox", "Windows Media Player"));
        assert!(!names_related("Google Chrome", "Application Verifier"));
        assert!(!names_related("C:\\Program Files", "App"));
    }

    #[test]
    fn best_name_match_never_attaches_a_wrong_app() {
        let cands = [(0usize, "Windows Media Player"), (1, "Roblox Player")];
        assert_eq!(best_name_match("Roblox", cands.into_iter()), Some(1));
    }

    #[test]
    fn best_name_match_returns_none_on_a_tie() {
        let cands = [(0usize, "Acme Foo"), (1, "Acme Bar")];
        assert_eq!(best_name_match("Acme", cands.into_iter()), None);
    }

    #[test]
    fn guid_like_names_are_detected() {
        assert!(looks_like_guid("db2168da-0bf0-5419-9f76-8b9058f9bde6"));
        assert!(!looks_like_guid("Roblox"));
        assert!(!looks_like_guid("db2168da-0bf0-5419-9f76"));
    }

    #[test]
    fn pe_arch_reads_the_real_header() {
        assert_eq!(pe_arch(Path::new("no-such-file-appsweep.exe")), None);
        let me = std::env::current_exe().expect("test exe");
        let arch = pe_arch(&me).expect("the test binary is a PE image");
        if cfg!(target_arch = "x86_64") {
            assert_eq!(arch, Arch::X64);
        }
    }

    /// A 6-byte PE stub is enough for `pe_arch`, which only reads the header.
    fn machine_of(machine: u16) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("appsweep-pe-{}.bin", machine));
        let mut f = std::fs::File::create(&path).expect("temp file");
        let mut head = [0u8; 0x40];
        head[..2].copy_from_slice(b"MZ");
        head[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        let mut body = b"PE\0\0".to_vec();
        body.extend_from_slice(&machine.to_le_bytes());
        let mut all = head.to_vec();
        all.extend_from_slice(&body);
        std::io::Write::write_all(&mut f, &all).expect("write");
        path
    }

    #[test]
    fn every_pe_machine_gets_its_own_arch() {
        let cases = [
            (0x014c, Arch::X86),
            (0x8664, Arch::X64),
            (0xaa64, Arch::Arm64),
            // ARMv7 Thumb-2 is 32-bit; calling it ARM64 was a real bug.
            (0x01c4, Arch::Arm32),
            (0x1234, Arch::Neutral),
        ];
        for (machine, want) in cases {
            let f = machine_of(machine);
            assert_eq!(pe_arch(&f), Some(want), "machine {:#06x}", machine);
            let _ = std::fs::remove_file(&f);
        }
    }

    #[test]
    fn a_file_without_a_pe_header_is_not_an_image() {
        let path = std::env::temp_dir().join("appsweep-pe-not-an-image.bin");
        std::fs::write(&path, b"not a pe file at all").expect("write");
        assert_eq!(pe_arch(&path), None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn commands_split_into_executable_and_arguments() {
        assert_eq!(
            split_command("\"C:\\Program Files\\App\\uninst.exe\" /S /quiet"),
            (
                "C:\\Program Files\\App\\uninst.exe".to_string(),
                "/S /quiet".to_string()
            )
        );
        assert_eq!(
            split_command("C:\\Tools\\setup.exe /silent"),
            ("C:\\Tools\\setup.exe".to_string(), "/silent".to_string())
        );
        assert_eq!(
            split_command("\"C:\\Tools\\a.exe\""),
            ("C:\\Tools\\a.exe".to_string(), String::new())
        );
        assert_eq!(
            split_command("C:\\Tools\\a.exe"),
            ("C:\\Tools\\a.exe".to_string(), String::new())
        );
        assert_eq!(split_command("  "), (String::new(), String::new()));
    }

    #[test]
    fn msi_product_codes_are_extracted() {
        let guid = "{1A2B3C4D-0000-1111-2222-333344445555}";
        let cmd = format!("MsiExec.exe /I{}", guid);
        assert_eq!(msi_product_from(&cmd).as_deref(), Some(guid));
        assert_eq!(msi_product_from("C:\\app\\uninstall.exe"), None);
        // A truncated brace pair must not be mistaken for a product code.
        assert_eq!(msi_product_from("msiexec /I{1234}"), None);
    }

    #[test]
    fn install_dates_are_normalised() {
        assert_eq!(parse_date("20240115"), "2024-01-15");
        assert_eq!(parse_date("2024/01/15"), "2024-01-15");
        assert_eq!(parse_date(""), "");
        assert_eq!(parse_date("2024"), "");
    }

    #[test]
    fn paths_are_compared_case_and_separator_insensitively() {
        assert!(is_under(
            "C:/Program Files/App/bin",
            "c:\\program files\\app"
        ));
        assert!(!is_under("C:\\Other", "C:\\Program Files"));
        // A shared prefix is not a parent folder.
        assert!(!is_under(
            "D:\\SharedToolsExtra\\beta.exe",
            "D:\\SharedTools"
        ));
        // Windows would resolve these to another folder; do not guess.
        assert!(!is_under(
            "D:\\SharedTools\\..\\Other\\beta.exe",
            "D:\\SharedTools"
        ));
        assert!(!is_under("D:\\SharedTools\\.\\beta.exe", "D:\\SharedTools"));
        assert!(is_under("D:\\SharedTools\\alpha.exe", "D:\\SharedTools"));
        assert!(is_under("D:\\SharedTools", "D:\\SharedTools\\"));
    }

    fn entry(id: u64, name: &str, source: Source) -> AppEntry {
        AppEntry::new(id, name.to_string(), source)
    }

    #[test]
    fn one_executable_with_two_registrations_keeps_both_keys() {
        let mut machine = entry(1, "Demo", Source::Registry);
        machine.launch_exe = Some(PathBuf::from("C:\\Demo\\demo.exe"));
        machine.reg_key = Some((RegRoot::Hklm, "Software\\Demo".to_string()));
        let mut user = entry(2, "Demo", Source::Registry);
        user.launch_exe = Some(PathBuf::from("C:\\Demo\\demo.exe"));
        user.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".to_string()));
        let out = dedupe(vec![machine, user]);
        assert_eq!(out.len(), 2, "two registrations are two removable entries");
        assert_eq!(
            out[0].reg_key,
            Some((RegRoot::Hklm, "Software\\Demo".to_string()))
        );
        assert_eq!(
            out[1].reg_key,
            Some((RegRoot::Hkcu, "Software\\Demo".to_string()))
        );
    }

    #[test]
    fn different_exes_in_one_folder_stay_separate() {
        let mut alpha = entry(1, "Alpha", Source::Shortcut);
        alpha.launch_exe = Some(PathBuf::from("D:\\SharedTools\\alpha.exe"));
        alpha.install_dir = Some(PathBuf::from("D:\\SharedTools"));
        let mut beta = entry(2, "Beta", Source::Shortcut);
        beta.launch_exe = Some(PathBuf::from("D:\\SharedTools\\beta.exe"));
        beta.install_dir = Some(PathBuf::from("D:\\SharedTools"));
        let out = dedupe(vec![alpha, beta]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "Alpha");
        assert_eq!(out[1].name, "Beta");
        assert_eq!(
            out[1].launch_exe.as_deref(),
            Some(Path::new("D:\\SharedTools\\beta.exe"))
        );
    }

    #[test]
    fn a_display_name_is_not_an_identity() {
        let mut first = entry(1, "Updater", Source::Registry);
        first.publisher = "Acme".to_string();
        first.reg_key = Some((RegRoot::Hkcu, "Software\\Acme\\Updater".to_string()));
        first.launch_exe = Some(PathBuf::from("C:\\Acme\\updater.exe"));
        let mut second = entry(2, "Updater", Source::Registry);
        second.publisher = "Other".to_string();
        second.reg_key = Some((RegRoot::Hkcu, "Software\\Other\\Updater".to_string()));
        second.launch_exe = Some(PathBuf::from("C:\\Other\\updater.exe"));
        let out = dedupe(vec![first, second]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].publisher, "Acme");
        assert_eq!(out[1].publisher, "Other");
    }

    #[test]
    fn shortcuts_never_attach_across_a_folder_boundary() {
        let mut app = entry(1, "Shared Tools", Source::Registry);
        app.launch_exe = Some(PathBuf::from("D:\\SharedTools\\alpha.exe"));
        app.install_dir = Some(PathBuf::from("D:\\SharedTools"));
        assert!(shortcut_target_belongs(&app, "D:/SharedTools/alpha.exe"));
        assert!(!shortcut_target_belongs(&app, "D:\\SharedTools\\beta.exe"));
        assert!(!shortcut_target_belongs(
            &app,
            "D:\\SharedToolsExtra\\beta.exe"
        ));
        // Without a known exe, an app may adopt any executable in its folder.
        app.launch_exe = None;
        assert!(shortcut_target_belongs(&app, "D:\\SharedTools\\beta.exe"));
        assert!(!shortcut_target_belongs(
            &app,
            "D:\\SharedToolsExtra\\beta.exe"
        ));
    }

    #[test]
    fn a_shortcut_and_its_registration_merge_and_keep_the_key() {
        let mut reg = entry(1, "Demo", Source::Registry);
        reg.launch_exe = Some(PathBuf::from("C:\\Demo\\demo.exe"));
        reg.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".to_string()));
        let mut link = entry(2, "Demo", Source::Shortcut);
        link.launch_exe = Some(PathBuf::from("C:\\Demo\\demo.exe"));
        link.install_dir = Some(PathBuf::from("C:\\Demo"));
        link.shortcuts = vec![PathBuf::from("C:\\Start\\Demo.lnk")];
        let out = dedupe(vec![reg, link]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].source, Source::Registry);
        assert_eq!(
            out[0].reg_key,
            Some((RegRoot::Hkcu, "Software\\Demo".to_string()))
        );
        assert_eq!(out[0].shortcuts.len(), 1);
    }

    #[test]
    fn store_logo_uris_become_relative_paths() {
        assert_eq!(
            ms_appx_relative("ms-appx://Package/Assets/Logo.png").as_deref(),
            Some("Assets\\Logo.png")
        );
        assert_eq!(ms_appx_relative("ms-resource:AppName"), None);
        assert_eq!(
            ms_appx_relative("C:\\Assets\\logo.png").as_deref(),
            Some("C:\\Assets\\logo.png")
        );
        assert_eq!(ms_appx_relative("logo.png"), None);
    }

    #[test]
    fn windows_folders_are_flagged_as_system() {
        let win = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        assert!(is_system_path(Path::new(&format!(
            "{}\\System32\\calc.exe",
            win
        ))));
        assert!(!is_system_path(Path::new("D:\\Apps\\Demo")));
        assert!(!is_system_path(Path::new(
            "C:\\Program Files\\WindowsApps\\App"
        )));
    }
}
