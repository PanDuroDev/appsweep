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
    pub fn label_ar(&self) -> &'static str {
        match self {
            Density::Compact => "مضغوط",
            Density::Normal => "عادي",
            Density::Relaxed => "واسع",
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
    pub fn label_ar(&self) -> &'static str {
        match self {
            SortKey::Name => "الاسم",
            SortKey::Publisher => "الناشر",
            SortKey::Size => "الحجم",
            SortKey::Date => "تاريخ التثبيت",
            SortKey::Source => "المصدر",
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

    pub fn label_ar(&self) -> &'static str {
        match self {
            Accent::Blue => "أزرق",
            Accent::Teal => "فيروزي",
            Accent::Violet => "بنفسجي",
            Accent::Amber => "عنبري",
            Accent::Rose => "وردي",
            Accent::Green => "أخضر",
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
