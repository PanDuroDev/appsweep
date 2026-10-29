//! Per-app user metadata (favorite, hidden, notes) that has to survive a
//! rescan and a restart. Entries are keyed on the app identity, never on the
//! scan id, which is reassigned on every scan.

use crate::model::{AppEntry, RegRoot, Source};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub notes: String,
}

impl Meta {
    fn is_empty(&self) -> bool {
        !self.favorite && !self.hidden && self.notes.is_empty()
    }
}

/// A stable name for one installed app. Registry entries use their key (the
/// key is what gets deleted), Store apps their package name, shortcuts their
/// folder; the display name is only the last resort.
pub fn key(app: &AppEntry) -> String {
    match app.source {
        Source::Registry => app
            .reg_key
            .as_ref()
            .map(|(root, path)| format!("reg:{}:{}", reg_root_text(*root), path.to_lowercase())),
        Source::Uwp => app
            .package_full_name
            .as_ref()
            .map(|p| format!("uwp:{}", p.to_lowercase())),
        Source::Shortcut => app
            .install_dir
            .as_ref()
            .map(|d| format!("dir:{}", d.to_string_lossy().to_lowercase())),
    }
    .unwrap_or_else(|| format!("name:{}", app.name.to_lowercase()))
}

fn reg_root_text(root: RegRoot) -> &'static str {
    match root {
        RegRoot::Hklm => "hklm",
        RegRoot::Hklm32 => "hklm-32",
        RegRoot::Hkcu => "hkcu",
    }
}

pub fn path() -> PathBuf {
    crate::icons::app_data_dir().join("apps.json")
}

pub fn load() -> BTreeMap<String, Meta> {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Records the current state of one app and writes the whole file back. A
/// missing metadata file is not an error, an unreadable one is replaced.
pub fn save(app: &AppEntry) {
    let mut all = load();
    let k = key(app);
    let meta = Meta {
        favorite: app.favorite,
        hidden: app.hidden,
        notes: app.notes.clone(),
    };
    if meta.is_empty() {
        all.remove(&k);
    } else {
        all.insert(k, meta);
    }
    if let Ok(text) = serde_json::to_string_pretty(&all) {
        let p = path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, text);
    }
}

/// Reapplies saved metadata to a freshly scanned list.
pub fn apply(apps: &mut [AppEntry]) {
    let all = load();
    if all.is_empty() {
        return;
    }
    for app in apps.iter_mut() {
        if let Some(m) = all.get(&key(app)) {
            app.favorite = m.favorite;
            app.hidden = m.hidden;
            app.notes = m.notes.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_does_not_use_the_scan_id() {
        let mut a = AppEntry::new(1, "Demo".into(), Source::Registry);
        a.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".into()));
        let first = key(&a);
        a.id = 999;
        a.name = "Demo renamed".into();
        assert_eq!(key(&a), first);
    }

    #[test]
    fn two_apps_with_the_same_name_do_not_share_metadata() {
        let mut a = AppEntry::new(1, "Demo".into(), Source::Registry);
        a.reg_key = Some((RegRoot::Hklm, "Software\\Demo".into()));
        let mut b = AppEntry::new(2, "Demo".into(), Source::Registry);
        b.reg_key = Some((RegRoot::Hklm32, "Software\\Demo".into()));
        assert_ne!(key(&a), key(&b));
    }

    #[test]
    fn the_32_bit_view_is_a_different_key() {
        let mut a = AppEntry::new(1, "Demo".into(), Source::Registry);
        a.reg_key = Some((RegRoot::Hklm, "Software\\Demo".into()));
        let mut b = AppEntry::new(1, "Demo".into(), Source::Registry);
        b.reg_key = Some((RegRoot::Hklm32, "Software\\Demo".into()));
        assert_ne!(key(&a), key(&b));
    }

    #[test]
    fn an_entry_without_identity_falls_back_to_its_name() {
        let a = AppEntry::new(7, "Loose".into(), Source::Uwp);
        assert_eq!(key(&a), "name:loose");
    }

    #[test]
    fn a_metadata_record_round_trips() {
        let m = Meta {
            favorite: true,
            hidden: true,
            notes: "keep".into(),
        };
        let text = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Meta>(&text).unwrap(), m);
        let loose: Meta = serde_json::from_str("{}").unwrap();
        assert!(loose.is_empty());
    }
}
