use std::path::PathBuf;

/// Folders that must never be deleted wholesale, no matter which app points
/// at them. A wrong install path in the registry must not wipe the system.
pub fn is_protected_dir(dir: &std::path::Path) -> bool {
    let norm = |p: &str| {
        p.trim_end_matches(['\\', '/'])
            .replace('/', "\\")
            .to_lowercase()
    };
    let target = norm(&dir.to_string_lossy());
    if target.is_empty() {
        return true;
    }
    // A drive root such as "c:" or "c:\".
    let bytes = target.as_bytes();
    if bytes.len() <= 2 || (bytes.len() == 3 && bytes[1] == b':' && bytes[2] == b'\\') {
        return true;
    }

    let mut guarded: Vec<String> = vec![
        r"c:\windows".into(),
        r"c:\windows\system32".into(),
        r"c:\windows\syswow64".into(),
        r"c:\windows\winsxs".into(),
        r"c:\program files".into(),
        r"c:\program files (x86)".into(),
        r"c:\programdata".into(),
        r"c:\users".into(),
        r"c:\users\public".into(),
        r"c:\$recycle.bin".into(),
        r"c:\system volume information".into(),
    ];
    for var in [
        "SystemRoot",
        "windir",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
        "SystemDrive",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PUBLIC",
        "HOMEDRIVE",
    ] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                guarded.push(norm(&v));
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            guarded.push(norm(&parent.to_string_lossy()));
        }
    }

    for g in guarded {
        if g.is_empty() {
            continue;
        }
        if target == g || g.starts_with(&format!("{}\\", target)) {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Registry,
    Uwp,
    Shortcut,
}

impl Source {
    pub fn label_ar(&self) -> &'static str {
        match self {
            Source::Registry => "برنامج سطح مكتب",
            Source::Uwp => "تطبيق متجر",
            Source::Shortcut => "اختصار",
        }
    }
    pub fn short_ar(&self) -> &'static str {
        match self {
            Source::Registry => "سطح المكتب",
            Source::Uwp => "متجر",
            Source::Shortcut => "اختصار",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arch {
    X64,
    X86,
    Arm64,
    Neutral,
}

impl Arch {
    pub fn text(&self) -> &'static str {
        match self {
            Arch::X64 => "x64",
            Arch::X86 => "x86",
            Arch::Arm64 => "ARM64",
            Arch::Neutral => "—",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub id: u64,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub source: Source,
    pub install_dir: Option<PathBuf>,
    /// File to open in Explorer (usually the main exe).
    pub launch_exe: Option<PathBuf>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub msi_product: Option<String>,
    pub package_full_name: Option<String>,
    pub install_date: String,
    pub size_bytes: u64,
    pub arch: Arch,
    pub is_system: bool,
    pub is_framework: bool,
    /// Registry key path, used to delete the entry.
    pub reg_key: Option<(RegRoot, String)>,
    /// Start menu shortcuts belonging to this app.
    pub shortcuts: Vec<PathBuf>,
    pub icon_path: Option<PathBuf>,
    /// Extra traces found on disk (AppData / ProgramData / temp).
    pub traces: Vec<PathBuf>,
    pub hidden: bool,
    pub favorite: bool,
    pub notes: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RegRoot {
    Hklm,
    Hklm32,
    Hkcu,
}

impl RegRoot {
    pub fn label(&self) -> &'static str {
        match self {
            RegRoot::Hklm => "HKLM",
            RegRoot::Hklm32 => "HKLM-32",
            RegRoot::Hkcu => "HKCU",
        }
    }
}

impl AppEntry {
    pub fn new(id: u64, name: String, source: Source) -> Self {
        Self {
            id,
            name,
            publisher: String::new(),
            version: String::new(),
            source,
            install_dir: None,
            launch_exe: None,
            uninstall_string: None,
            quiet_uninstall_string: None,
            msi_product: None,
            package_full_name: None,
            install_date: String::new(),
            size_bytes: 0,
            arch: Arch::Neutral,
            is_system: false,
            is_framework: false,
            reg_key: None,
            shortcuts: Vec::new(),
            icon_path: None,
            traces: Vec::new(),
            hidden: false,
            favorite: false,
            notes: String::new(),
        }
    }

    /// Best path to reveal in the file manager.
    pub fn reveal_path(&self) -> Option<PathBuf> {
        if let Some(exe) = &self.launch_exe {
            if exe.exists() {
                return Some(exe.clone());
            }
        }
        if let Some(dir) = &self.install_dir {
            if dir.exists() {
                return Some(dir.clone());
            }
        }
        self.icon_path.clone().filter(|p| p.exists())
    }

    /// True when the registry still lists the app but nothing is left on disk,
    /// which usually means an uninstall failed halfway.
    pub fn is_remnant(&self) -> bool {
        if self.source != Source::Registry {
            return false;
        }
        let dir_gone = self.install_dir.as_ref().map(|d| !d.exists()).unwrap_or(true);
        let exe_gone = self.launch_exe.as_ref().map(|e| !e.exists()).unwrap_or(true);
        dir_gone && exe_gone && self.icon_path.is_none()
    }

    /// Returns the Arabic reason when deleting this entry would be dangerous.
    /// Protects Windows itself, drive roots and shared system folders.
    pub fn blocked_reason(&self) -> Option<&'static str> {
        let dir = self.install_dir.as_ref()?;
        if is_protected_dir(dir) {
            return Some("مجلد محمي من النظام");
        }
        None
    }

    /// Path used for icon lookup. Never falls back to a folder, because that
    /// only yields a generic folder icon.
    pub fn icon_source(&self) -> Option<String> {
        if let Some(p) = &self.icon_path {
            if p.is_file() {
                return Some(p.to_string_lossy().to_string());
            }
        }
        if let Some(p) = &self.launch_exe {
            if p.is_file() {
                return Some(p.to_string_lossy().to_string());
            }
        }
        // A shortcut file carries the icon of its target, which is the best
        // fallback when the executable itself is missing or inaccessible.
        for s in &self.shortcuts {
            if s.is_file() {
                return Some(s.to_string_lossy().to_string());
            }
        }
        None
    }
}

/// Simple fuzzy-ish matcher used by the search box.
pub fn matches(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let h = haystack.to_lowercase();
    let n = needle.to_lowercase();
    if h.contains(&n) {
        return true;
    }
    // subsequence match
    let mut it = h.chars();
    n.chars().all(|c| it.any(|h| h == c))
}

pub fn human_size(bytes: u64) -> String {
    if bytes == 0 {
        return "—".to_string();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, UNITS[i])
    } else if v >= 100.0 {
        format!("{:.0} {}", v, UNITS[i])
    } else if v >= 10.0 {
        format!("{:.1} {}", v, UNITS[i])
    } else {
        format!("{:.2} {}", v, UNITS[i])
    }
}
