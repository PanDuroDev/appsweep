use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ViewMode {
    Cards,
    Table,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Density {
    Compact,
    Normal,
    Relaxed,
}

impl Density {
    pub fn row_height(&self) -> f32 {
        match self {
            Density::Compact => 26.0,
            Density::Normal => 34.0,
            Density::Relaxed => 44.0,
        }
    }
    pub fn spacing(&self) -> f32 {
        match self {
            Density::Compact => 4.0,
            Density::Normal => 8.0,
            Density::Relaxed => 14.0,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Density::Compact => "Compact",
            Density::Normal => "Normal",
            Density::Relaxed => "Relaxed",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SortKey {
    Name,
    Publisher,
    Size,
    Date,
    Source,
}

impl SortKey {
    pub fn label(&self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Publisher => "Publisher",
            SortKey::Size => "Size",
            SortKey::Date => "Install date",
            SortKey::Source => "Source",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Accent {
    Blue,
    Teal,
    Violet,
    Amber,
    Rose,
    Green,
}

impl Accent {
    pub const ALL: [Accent; 6] = [
        Accent::Blue,
        Accent::Teal,
        Accent::Violet,
        Accent::Amber,
        Accent::Rose,
        Accent::Green,
    ];

    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            Accent::Blue => (56, 132, 255),
            Accent::Teal => (32, 178, 170),
            Accent::Violet => (139, 108, 246),
            Accent::Amber => (233, 168, 60),
            Accent::Rose => (232, 93, 124),
            Accent::Green => (58, 183, 120),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Accent::Blue => "Blue",
            Accent::Teal => "Teal",
            Accent::Violet => "Violet",
            Accent::Amber => "Amber",
            Accent::Rose => "Rose",
            Accent::Green => "Green",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: ThemeMode,
    pub accent: Accent,
    pub density: Density,
    pub view: ViewMode,
    pub card_min_width: f32,
    pub icon_size: f32,
    pub show_system: bool,
    pub show_frameworks: bool,
    pub show_shortcuts: bool,
    pub sort: SortKey,
    pub sort_desc: bool,
    pub auto_cascade: bool,
    pub create_restore_point: bool,
    pub backup_registry: bool,
    pub kill_processes: bool,
    pub scan_on_start: bool,
    pub confirm_text: String,
    pub last_folder: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            accent: Accent::Blue,
            density: Density::Normal,
            view: ViewMode::Cards,
            card_min_width: 320.0,
            icon_size: 48.0,
            show_system: true,
            show_frameworks: false,
            show_shortcuts: true,
            sort: SortKey::Name,
            sort_desc: false,
            auto_cascade: true,
            create_restore_point: true,
            backup_registry: true,
            kill_processes: true,
            scan_on_start: true,
            confirm_text: String::new(),
            last_folder: String::new(),
        }
    }
}

impl Settings {
    pub fn path() -> PathBuf {
        crate::icons::app_data_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let p = Self::path();
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(s) = serde_json::from_str::<Settings>(&text) {
                return s;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_the_safety_nets_on() {
        let s = Settings::default();
        assert!(s.backup_registry);
        assert!(s.create_restore_point);
        assert!(s.kill_processes);
        assert!(s.auto_cascade);
        // Listing system apps was an explicit requirement.
        assert!(s.show_system);
        assert_eq!(s.theme, ThemeMode::Dark);
        assert_eq!(s.accent, Accent::Blue);
        assert!(!s.sort_desc);
        assert!((24.0..=96.0).contains(&s.icon_size));
        assert!(s.card_min_width >= 240.0);
    }

    #[test]
    fn every_field_survives_a_json_round_trip() {
        let s = Settings {
            theme: ThemeMode::Light,
            accent: Accent::Green,
            density: Density::Compact,
            view: ViewMode::Table,
            card_min_width: 412.0,
            icon_size: 64.0,
            show_system: false,
            show_frameworks: true,
            show_shortcuts: false,
            sort: SortKey::Size,
            sort_desc: true,
            auto_cascade: false,
            create_restore_point: false,
            backup_registry: false,
            kill_processes: false,
            scan_on_start: false,
            confirm_text: "x".into(),
            last_folder: "D:\\Apps".into(),
        };
        let text = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(back.theme, s.theme);
        assert_eq!(back.accent, s.accent);
        assert_eq!(back.density, s.density);
        assert_eq!(back.view, s.view);
        assert_eq!(back.card_min_width, s.card_min_width);
        assert_eq!(back.icon_size, s.icon_size);
        assert_eq!(back.show_system, s.show_system);
        assert_eq!(back.show_frameworks, s.show_frameworks);
        assert_eq!(back.show_shortcuts, s.show_shortcuts);
        assert_eq!(back.sort, s.sort);
        assert_eq!(back.sort_desc, s.sort_desc);
        assert_eq!(back.auto_cascade, s.auto_cascade);
        assert_eq!(back.create_restore_point, s.create_restore_point);
        assert_eq!(back.backup_registry, s.backup_registry);
        assert_eq!(back.kill_processes, s.kill_processes);
        assert_eq!(back.scan_on_start, s.scan_on_start);
        assert_eq!(back.confirm_text, s.confirm_text);
        assert_eq!(back.last_folder, s.last_folder);
    }

    #[test]
    fn a_broken_settings_file_is_rejected_not_half_applied() {
        // `load` falls back to `default()` when parsing fails, so every one of
        // these must be an error rather than a partly-filled struct.
        assert!(serde_json::from_str::<Settings>("{}").is_err());
        assert!(serde_json::from_str::<Settings>("not json").is_err());
        assert!(serde_json::from_str::<Settings>(r#"{"theme":"Neon"}"#).is_err());
    }

    #[test]
    fn density_and_accent_tables_are_complete() {
        assert!(Density::Compact.row_height() < Density::Normal.row_height());
        assert!(Density::Normal.row_height() < Density::Relaxed.row_height());
        assert!(Density::Compact.spacing() < Density::Relaxed.spacing());
        for d in [Density::Compact, Density::Normal, Density::Relaxed] {
            assert!(!d.label().is_empty());
        }
        let mut seen = std::collections::HashSet::new();
        for a in Accent::ALL {
            assert!(seen.insert(a.rgb()), "duplicate accent colour");
            assert!(!a.label().is_empty());
        }
        assert_eq!(seen.len(), Accent::ALL.len());
    }
}
