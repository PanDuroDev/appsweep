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

    for g in &guarded {
        if g.is_empty() {
            continue;
        }
        if target == *g || g.starts_with(&format!("{}\\", target)) {
            return true;
        }
    }
    // Inside Windows nothing is an app folder, so the whole subtree is off
    // limits. Program Files, Users and ProgramData keep only the shallow rule
    // above: apps really do live one level down in those.
    let mut deep: Vec<String> = vec![r"c:\windows".into()];
    for var in ["SystemRoot", "windir"] {
        if let Ok(v) = std::env::var(var) {
            deep.push(norm(&v));
        }
    }
    deep.iter()
        .any(|g| !g.is_empty() && target.starts_with(&format!("{}\\", g)))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Registry,
    Uwp,
    Shortcut,
}

impl Source {
    pub fn label(&self) -> &'static str {
        match self {
            Source::Registry => "Desktop program",
            Source::Uwp => "Store app",
            Source::Shortcut => "Shortcut",
        }
    }
    pub fn short(&self) -> &'static str {
        match self {
            Source::Registry => "Desktop",
            Source::Uwp => "Store",
            Source::Shortcut => "Shortcut",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arch {
    X64,
    X86,
    Arm32,
    Arm64,
    Neutral,
}

impl Arch {
    pub fn text(&self) -> &'static str {
        match self {
            Arch::X64 => "x64",
            Arch::X86 => "x86",
            Arch::Arm32 => "ARM32",
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
        let dir_gone = self
            .install_dir
            .as_ref()
            .map(|d| !d.exists())
            .unwrap_or(true);
        let exe_gone = self
            .launch_exe
            .as_ref()
            .map(|e| !e.exists())
            .unwrap_or(true);
        // The icon is not evidence either way: the scanner stores the launch
        // exe (or the uninstaller) as the icon source, so an entry with a dead
        // path always has one and was never reported as a remnant.
        dir_gone && exe_gone
    }

    /// Returns the Arabic reason when deleting this entry would be dangerous.
    /// Protects Windows itself, drive roots and shared system folders.
    pub fn blocked_reason(&self) -> Option<&'static str> {
        let dir = self.install_dir.as_ref()?;
        if is_protected_dir(dir) {
            return Some("System-protected folder");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn search_matches_substrings_and_subsequences() {
        assert!(matches("Google Chrome", "chrome"));
        assert!(matches("Google Chrome", "gch"));
        assert!(matches("anything", ""));
        assert!(!matches("Google Chrome", "firefox"));
    }

    #[test]
    fn human_size_uses_binary_units() {
        assert_eq!(human_size(0), "—");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.00 KB");
        assert_eq!(human_size(100 * 1024 * 1024), "100 MB");
        assert_eq!(human_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn protected_dirs_are_never_deletable() {
        for p in [
            r"c:\",
            r"C:\Windows",
            r"C:\Windows\System32",
            r"C:\Program Files",
            r"C:\Program Files (x86)",
            r"C:\ProgramData",
            r"C:\Users",
        ] {
            assert!(is_protected_dir(Path::new(p)), "{p} must be protected");
        }
        assert!(!is_protected_dir(Path::new(r"D:\Apps\Demo")));
        assert!(!is_protected_dir(Path::new(r"C:\Program Files\7-Zip")));
    }

    #[test]
    fn nothing_inside_windows_is_an_app_folder() {
        // A registry entry pointing at a Windows subfolder used to pass the
        // guard: only the Windows folder itself was blocked, not its children.
        for p in [
            r"C:\Windows\System32\drivers",
            r"C:\Windows\System32\DriverStore\FileRepository",
            r"C:\Windows\Temp\SomeApp",
            r"C:\Windows\assembly",
        ] {
            assert!(is_protected_dir(Path::new(p)), "{p} must be protected");
        }
        // A sibling folder whose name merely starts with "Windows" is an app.
        assert!(!is_protected_dir(Path::new(r"C:\WindowsApps\Demo")));
    }

    #[test]
    fn remnant_needs_a_missing_install() {
        let mut e = AppEntry::new(1, "Gone".into(), Source::Registry);
        assert!(e.is_remnant());
        e.source = Source::Uwp;
        assert!(!e.is_remnant());
        let mut e = AppEntry::new(2, "Here".into(), Source::Registry);
        e.install_dir = Some(std::env::temp_dir());
        assert!(!e.is_remnant());
    }

    #[test]
    fn a_stale_icon_path_does_not_hide_a_remnant() {
        // The scanner copies the launch exe into `icon_path`, so a dead install
        // used to look alive only because that string was still set.
        let mut e = AppEntry::new(3, "Orphan".into(), Source::Registry);
        let gone = |name: &str| std::env::temp_dir().join(format!("appsweep-gone-{name}"));
        e.install_dir = Some(gone("dir"));
        e.launch_exe = Some(gone("app.exe"));
        e.icon_path = Some(gone("app.exe"));
        assert!(e.is_remnant());
    }

    #[test]
    fn every_enum_variant_has_a_label() {
        for a in [
            Arch::X64,
            Arch::X86,
            Arch::Arm32,
            Arch::Arm64,
            Arch::Neutral,
        ] {
            assert!(!a.text().is_empty());
        }
        // ARMv7 must not be shown as ARM64: one is 32-bit, the other is not.
        assert_eq!(Arch::Arm32.text(), "ARM32");
        assert_ne!(Arch::Arm32.text(), Arch::Arm64.text());
        for s in [Source::Registry, Source::Uwp, Source::Shortcut] {
            assert!(!s.label().is_empty());
            assert!(!s.short().is_empty());
        }
        assert_eq!(RegRoot::Hklm.label(), "HKLM");
        assert_eq!(RegRoot::Hklm32.label(), "HKLM-32");
        assert_eq!(RegRoot::Hkcu.label(), "HKCU");
    }

    #[test]
    fn a_new_entry_starts_empty_but_complete() {
        let e = AppEntry::new(7, "Demo".into(), Source::Registry);
        assert_eq!(e.id, 7);
        assert!(!e.hidden && !e.favorite && !e.is_system && !e.is_framework);
        assert!(e.shortcuts.is_empty() && e.traces.is_empty());
        assert!(e.notes.is_empty() && e.package_full_name.is_none());
        assert_eq!(e.arch, Arch::Neutral);
        assert_eq!(e.reveal_path(), None);
    }
}
