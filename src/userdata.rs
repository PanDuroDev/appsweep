//! Per-app user metadata (favorite, hidden, notes) that has to survive a
//! rescan and a restart. Entries are keyed on the app identity, never on the
//! scan id, which is reassigned on every scan.

use crate::model::{AppEntry, RegRoot, Source};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
/// key is what gets deleted), Store apps their package family, shortcuts their
/// folder; the display name is only the last resort.
///
/// shortcut: two entries with no identity at all still share the name key.
/// A separate identity for them needs a scanner-side id, not a name.
pub fn key(app: &AppEntry) -> String {
    match app.source {
        Source::Registry => app
            .reg_key
            .as_ref()
            .map(|(root, path)| format!("reg:{}:{}", reg_root_text(*root), path.to_lowercase())),
        Source::Uwp => app
            .package_full_name
            .as_deref()
            .map(|p| format!("uwp:{}", package_family(p).to_lowercase())),
        Source::Shortcut => app
            .install_dir
            .as_ref()
            .map(|d| format!("dir:{}", d.to_string_lossy().to_lowercase())),
    }
    .unwrap_or_else(|| format!("name:{}", app.name.to_lowercase()))
}

/// Older builds keyed Store apps by the full package name, which changes with
/// every version. Existing records are still read under that key.
fn legacy_key(app: &AppEntry) -> Option<String> {
    if app.source != Source::Uwp {
        return None;
    }
    app.package_full_name
        .as_deref()
        .map(|p| format!("uwp:{}", p.to_lowercase()))
}

/// `Name_Version_Arch_ResourceId_PublisherId` -> `Name_PublisherId`, the
/// package family, which stays the same across package updates.
fn package_family(full: &str) -> String {
    let parts: Vec<&str> = full.split('_').collect();
    if parts.len() == 5 {
        format!("{}_{}", parts[0], parts[4])
    } else {
        full.to_string()
    }
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

/// A missing file is an empty store. A file that cannot be read or parsed is
/// an error: it must never be replaced by whatever the caller had in mind.
pub fn load_from(p: &Path) -> Result<BTreeMap<String, Meta>, String> {
    match std::fs::read_to_string(p) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} is not valid JSON ({e}); it was left as is", p.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(format!("Could not read {}: {e}", p.display())),
    }
}

/// Writes the new document next to the old one and swaps it in, so an
/// interrupted write leaves the previous file intact.
fn write_atomic(p: &Path, text: &str) -> Result<(), String> {
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, p).map_err(|e| format!("Could not replace {}: {e}", p.display()))
}

/// Serialises read-modify-write between two AppSweep windows.
fn lock(p: &Path) -> Result<std::fs::File, String> {
    let lock_path = p.with_extension("lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|e| format!("Could not open {}: {e}", lock_path.display()))?;
    file.lock()
        .map_err(|e| format!("Could not lock {}: {e}", lock_path.display()))?;
    Ok(file)
}

pub fn save_to(p: &Path, app: &AppEntry) -> Result<(), String> {
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    }
    // Held until the end of the function; dropping the file releases the lock.
    let _guard = lock(p)?;
    let mut all = load_from(p)?;
    let k = key(app);
    if let Some(legacy) = legacy_key(app) {
        all.remove(&legacy);
    }
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
    let text = serde_json::to_string_pretty(&all)
        .map_err(|e| format!("Could not encode {}: {e}", p.display()))?;
    write_atomic(p, &text)
}

pub fn save(app: &AppEntry) -> Result<(), String> {
    save_to(&path(), app)
}

/// Reapplies saved metadata to a freshly scanned list.
pub fn apply(apps: &mut [AppEntry]) -> Result<(), String> {
    apply_from(&path(), apps)
}

fn apply_from(p: &Path, apps: &mut [AppEntry]) -> Result<(), String> {
    let all = load_from(p)?;
    if all.is_empty() {
        return Ok(());
    }
    for app in apps.iter_mut() {
        let found = all
            .get(&key(app))
            .or_else(|| legacy_key(app).and_then(|k| all.get(&k)));
        if let Some(m) = found {
            app.favorite = m.favorite;
            app.hidden = m.hidden;
            app.notes = m.notes.clone();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        loop {
            let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir()
                .join(format!("appsweep-meta-{tag}-{}-{id}", std::process::id()));
            match std::fs::create_dir(&p) {
                Ok(()) => return p,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("Could not create {}: {e}", p.display()),
            }
        }
    }

    fn cleanup_temp_dir(dir: &Path) {
        assert!(
            !crate::model::is_protected_dir(dir),
            "Refusing to remove protected directory {}",
            dir.display()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn store_app(full: &str) -> AppEntry {
        let mut a = AppEntry::new(1, "Demo".into(), Source::Uwp);
        a.package_full_name = Some(full.into());
        a
    }

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
    fn a_package_update_keeps_the_same_metadata_key() {
        let old = store_app("MSTeams_26260.1717_x64__8wekyb3d8bbwe");
        let new = store_app("MSTeams_26300.1000_x64__8wekyb3d8bbwe");
        assert_eq!(key(&old), key(&new));
        assert_eq!(key(&old), "uwp:msteams_8wekyb3d8bbwe");
        let other = store_app("MSTeams_26260.1717_x64__9zzzzzzzzzzzz");
        assert_ne!(key(&old), key(&other));
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

    #[test]
    fn a_missing_file_is_an_empty_store() {
        let dir = temp_dir("missing");
        assert!(load_from(&dir.join("apps.json")).unwrap().is_empty());
    }

    #[test]
    fn corrupt_metadata_is_never_overwritten() {
        let dir = temp_dir("corrupt");
        let p = dir.join("apps.json");
        std::fs::write(&p, "{ not json").unwrap();
        let mut app = AppEntry::new(1, "Demo".into(), Source::Registry);
        app.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".into()));
        app.favorite = true;
        assert!(save_to(&p, &app).is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "{ not json");
    }

    #[test]
    fn two_saves_keep_both_records() {
        let dir = temp_dir("twosaves");
        let p = dir.join("apps.json");
        let mut a = AppEntry::new(1, "A".into(), Source::Registry);
        a.reg_key = Some((RegRoot::Hkcu, "Software\\A".into()));
        a.favorite = true;
        let mut b = AppEntry::new(2, "B".into(), Source::Registry);
        b.reg_key = Some((RegRoot::Hkcu, "Software\\B".into()));
        b.notes = "note".into();
        save_to(&p, &a).unwrap();
        save_to(&p, &b).unwrap();
        let all = load_from(&p).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all[&key(&a)].favorite);
        assert_eq!(all[&key(&b)].notes, "note");
    }

    #[test]
    fn clearing_metadata_removes_legacy_record_before_apply() {
        let dir = temp_dir("clear-legacy");
        let p = dir.join("apps.json");
        let app = store_app("MSTeams_26260.1717_x64__8wekyb3d8bbwe");
        let legacy = legacy_key(&app).unwrap();
        let mut all = BTreeMap::new();
        all.insert(
            legacy.clone(),
            Meta {
                hidden: true,
                favorite: true,
                notes: "old".into(),
            },
        );
        all.insert(
            "name:another".into(),
            Meta {
                favorite: true,
                ..Meta::default()
            },
        );
        std::fs::write(&p, serde_json::to_string(&all).unwrap()).unwrap();

        let mut before = [app.clone()];
        apply_from(&p, &mut before).unwrap();
        assert!(before[0].hidden);
        save_to(&p, &app).unwrap();
        let all = load_from(&p).unwrap();
        assert!(!all.contains_key(&legacy));
        assert!(all["name:another"].favorite);
        let mut apps = [app];
        apply_from(&p, &mut apps).unwrap();
        assert!(!apps[0].hidden);
        assert!(!apps[0].favorite);
        assert!(apps[0].notes.is_empty());
        cleanup_temp_dir(&dir);
    }

    #[test]
    fn saving_metadata_replaces_legacy_record_even_when_keys_match() {
        let dir = temp_dir("replace-legacy");
        let p = dir.join("apps.json");
        for full in ["MSTeams_26260.1717_x64__8wekyb3d8bbwe", "family"] {
            let mut app = store_app(full);
            let legacy = legacy_key(&app).unwrap();
            let mut all = BTreeMap::new();
            all.insert(
                legacy.clone(),
                Meta {
                    hidden: true,
                    ..Meta::default()
                },
            );
            std::fs::write(&p, serde_json::to_string(&all).unwrap()).unwrap();
            app.favorite = true;

            save_to(&p, &app).unwrap();
            let all = load_from(&p).unwrap();
            assert_eq!(all.len(), 1);
            assert!(all[&key(&app)].favorite);
            assert!(!all[&key(&app)].hidden);
            if legacy != key(&app) {
                assert!(!all.contains_key(&legacy));
            }
        }
        cleanup_temp_dir(&dir);
    }

    #[test]
    fn a_completed_save_leaves_no_partial_file() {
        let dir = temp_dir("atomic");
        let p = dir.join("apps.json");
        let mut app = AppEntry::new(1, "Demo".into(), Source::Registry);
        app.reg_key = Some((RegRoot::Hkcu, "Software\\Demo".into()));
        app.favorite = true;
        save_to(&p, &app).unwrap();
        assert!(!p.with_extension("tmp").exists());
        assert!(load_from(&p).unwrap().contains_key(&key(&app)));
    }
}
