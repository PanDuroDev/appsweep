//! The application shell: header, list area, status bar and the modal stack.

pub mod brand;
pub mod cards;
pub mod dialogs;
pub mod glyph;
pub mod table;
pub mod widgets;

use crate::actions::{self, Flow, Job, StepStatus};
use crate::icons::{IconCache, IconService};
use crate::model::{human_size, matches, AppEntry, Source};
use crate::settings::{Settings, SortKey, ViewMode};
use crate::theme::{self, Palette};
use dialogs::{Asked, ConfirmState, Filter};
use egui::{
    vec2, Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, RichText, Sense, Stroke, Ui,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;
use widgets::{Act, Ctx};

pub struct AppSweep {
    settings: Settings,
    palette: Palette,
    apps: Vec<AppEntry>,
    icons: IconService,
    cache: IconCache,
    scanning: bool,
    scan_rx: Option<Receiver<crate::scan::ScanResult>>,
    enrich_rx: Option<Receiver<Vec<crate::scan::Enrich>>>,
    enriching: bool,
    warnings: Vec<String>,
    search: String,
    filter: Filter,
    selected: Option<u64>,
    job: Option<Job>,
    confirm: Option<ConfirmState>,
    details: Option<u64>,
    show_settings: bool,
    show_backups: bool,
    backups: Vec<crate::actions::BackupEntry>,
    confirm_backup_clear: bool,
    elevated: bool,
    status: String,
    status_at: Option<Instant>,
    scan_at: Option<Instant>,
    scanned_once: bool,
    open_on_scan: Option<String>,
    pub view: ViewMode,
}

impl AppSweep {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        let palette = Palette::new(settings.theme, settings.accent);
        let mut app = Self {
            view: settings.view,
            settings,
            palette,
            apps: Vec::new(),
            icons: IconService::new(cc.egui_ctx.clone()),
            cache: IconCache::default(),
            scanning: false,
            scan_rx: None,
            enrich_rx: None,
            enriching: false,
            warnings: Vec::new(),
            search: String::new(),
            filter: Filter::All,
            selected: None,
            job: None,
            confirm: None,
            details: None,
            show_settings: false,
            show_backups: false,
            backups: Vec::new(),
            confirm_backup_clear: false,
            elevated: crate::native::is_elevated(),
            status: String::new(),
            status_at: None,
            scan_at: None,
            scanned_once: false,
            open_on_scan: None,
        };
        if app.settings.scan_on_start {
            app.start_scan(&cc.egui_ctx);
        }
        // Release build: never auto-open a dialog from the environment. An
        // automatic purge dialog must be impossible outside debug builds.
        if cfg!(debug_assertions) {
            app.open_on_scan = std::env::var("APPSWEEP_OPEN").ok();
        }
        app
    }

    fn start_scan(&mut self, ctx: &egui::Context) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        self.enrich_rx = None;
        self.enriching = false;
        let (tx, rx) = std::sync::mpsc::channel();
        self.scan_rx = Some(rx);
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new()
            .name("scan".into())
            .spawn(move || {
                let res = crate::scan::scan_all();
                let _ = tx.send(res);
                ctx.request_repaint();
            });
    }

    /// Logos and folder sizes are slow, so they stream in after the list is up.
    fn start_enrich(&mut self, ctx: &egui::Context) {
        if self.enriching || self.apps.is_empty() {
            return;
        }
        self.enriching = true;
        let (tx, rx) = std::sync::mpsc::channel();
        self.enrich_rx = Some(rx);
        let apps = self.apps.clone();
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new()
            .name("enrich".into())
            .spawn(move || {
                crate::scan::enrich_apps(&apps, move |batch| {
                    let _ = tx.send(batch);
                    ctx.request_repaint();
                });
            });
    }

    fn apply_enrich(&mut self, batch: Vec<crate::scan::Enrich>) {
        for e in batch {
            if let Some(app) = self.apps.iter_mut().find(|a| a.id == e.id) {
                if app.icon_path.is_none() {
                    if let Some(i) = e.icon {
                        app.icon_path = Some(i);
                    }
                }
                if app.size_bytes == 0 && e.size > 0 {
                    app.size_bytes = e.size;
                }
            }
        }
    }
    fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_at = Some(Instant::now());
    }

    /// Dev aid used by the screenshot scripts.
    fn debug_open(&mut self) {
        let what = match self.open_on_scan.take() {
            Some(w) => w,
            None => return,
        };
        let first = match self.apps.first() {
            Some(a) => a.clone(),
            None => return,
        };
        match what.as_str() {
            "settings" => self.show_settings = true,
            "backups" => {
                self.backups = crate::actions::list_backups();
                self.show_backups = true;
            }
            "details" => {
                self.selected = Some(first.id);
                self.details = Some(first.id);
            }
            "confirm" => {
                self.confirm = Some(ConfirmState {
                    app: first.clone(),
                    flow: Flow::Uninstall,
                    stage: 0,
                    ack: false,
                    auto_cascade: self.settings.auto_cascade,
                    keep_data: true,
                    typed: String::new(),
                });
            }
            "purge" => {
                self.confirm = Some(ConfirmState {
                    app: first.clone(),
                    flow: Flow::Purge,
                    stage: 1,
                    ack: false,
                    auto_cascade: self.settings.auto_cascade,
                    keep_data: false,
                    typed: String::new(),
                });
            }
            _ => {}
        }
    }

    fn filtered(&self) -> Vec<&AppEntry> {
        let mut out: Vec<&AppEntry> = self
            .apps
            .iter()
            .filter(|a| {
                if a.hidden {
                    return false;
                }
                if a.is_system && !self.settings.show_system {
                    return false;
                }
                if a.is_framework && !self.settings.show_frameworks {
                    return false;
                }
                if a.source == Source::Shortcut && !self.settings.show_shortcuts {
                    return false;
                }
                match self.filter {
                    Filter::All => true,
                    Filter::Desktop => a.source == Source::Registry,
                    Filter::Store => a.source == Source::Uwp,
                    Filter::System => a.is_system,
                    Filter::Favorites => a.favorite,
                    Filter::Remnants => a.is_remnant(),
                }
            })
            .filter(|a| {
                if self.search.trim().is_empty() {
                    return true;
                }
                let q = self.search.trim();
                // A query of "#<digits>" (or bare digits) looks up the numeric id.
                let digits: String = q.trim_start_matches('#').to_string();
                if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(n) = digits.parse::<u64>() {
                        if n == a.id {
                            return true;
                        }
                    }
                }
                if matches(&a.name, q) || matches(&a.publisher, q) {
                    return true;
                }
                if let Some(d) = &a.install_dir {
                    if matches(&d.to_string_lossy(), q) {
                        return true;
                    }
                }
                matches(&a.version, q)
            })
            .collect();

        let desc = self.settings.sort_desc;
        match self.settings.sort {
            SortKey::Name => out.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then(a.publisher.cmp(&b.publisher))
            }),
            SortKey::Publisher => out.sort_by(|a, b| {
                a.publisher
                    .to_lowercase()
                    .cmp(&b.publisher.to_lowercase())
                    .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }),
            SortKey::Size => out.sort_by_key(|a| a.size_bytes),
            SortKey::Date => out.sort_by(|a, b| a.install_date.cmp(&b.install_date)),
            SortKey::Source => out.sort_by(|a, b| {
                (a.source as u8)
                    .cmp(&(b.source as u8))
                    .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }),
        }
        if desc {
            out.reverse();
        }
        // Favourites always float to the top of the list.
        out.sort_by_key(|a| !a.favorite);
        out
    }

    fn find(&self, id: u64) -> Option<&AppEntry> {
        self.apps.iter().find(|a| a.id == id)
    }

    /// Opens the confirmation dialog, refusing entries whose folder is a
    /// protected system location.
    fn ask_delete(&mut self, id: u64, flow: Flow, stage: u8, keep_data: bool) {
        let (app, blocked, name) = match self.find(id) {
            Some(a) => (a.clone(), a.blocked_reason(), a.name.clone()),
            None => return,
        };
        if let Some(reason) = blocked {
            let msg = format!("Cannot delete {} — {}", name, reason);
            self.set_status(msg);
            return;
        }
        self.confirm = Some(ConfirmState {
            app,
            flow,
            stage,
            ack: false,
            auto_cascade: self.settings.auto_cascade,
            keep_data,
            typed: String::new(),
        });
    }

    fn handle(&mut self, act: Act, ctx: &egui::Context) {
        match act {
            Act::Select(id) => {
                self.selected = if self.selected == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
            Act::Uninstall(id) => self.ask_delete(id, Flow::Uninstall, 0, true),
            Act::Purge(id) => self.ask_delete(id, Flow::Purge, 1, false),
            Act::Files(id) => self.ask_delete(id, Flow::FilesOnly, 0, false),
            Act::Details(id) => {
                self.selected = Some(id);
                // Trace discovery walks AppData/ProgramData, so do it once
                // when the sheet opens instead of on every frame.
                if let Some(app) = self.apps.iter_mut().find(|a| a.id == id) {
                    if app.traces.is_empty() {
                        app.traces = crate::actions::find_traces(app);
                    }
                }
                self.details = Some(id);
            }
            Act::Reveal(id) => {
                if let Some(app) = self.find(id) {
                    match app.reveal_path() {
                        Some(p) => {
                            let s = p.to_string_lossy().to_string();
                            let select = p.is_file();
                            if crate::native::open_in_explorer(&s, select) != 0 {
                                self.set_status("Could not open File Explorer");
                            }
                        }
                        None => self.set_status("No known path for this app"),
                    }
                }
            }
            Act::Props(id) => {
                if let Some(app) = self.find(id) {
                    if let Some(p) = app.reveal_path() {
                        crate::native::show_properties(&p.to_string_lossy());
                    }
                }
            }
            Act::CopyPath(id) => {
                if let Some(app) = self.find(id) {
                    let text = app
                        .reveal_path()
                        .map(|p| p.to_string_lossy().to_string())
                        .or_else(|| {
                            app.install_dir
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                        });
                    if let Some(t) = text {
                        ctx.copy_text(t);
                        self.set_status("Path copied");
                    }
                }
            }
            Act::Favorite(id) => {
                let mut changed = None;
                if let Some(app) = self.apps.iter_mut().find(|a| a.id == id) {
                    app.favorite = !app.favorite;
                    changed = Some(app.clone());
                }
                if let Some(app) = changed {
                    crate::userdata::save(&app);
                }
            }
            Act::Hide(id) => {
                let mut changed = None;
                if let Some(app) = self.apps.iter_mut().find(|a| a.id == id) {
                    app.hidden = true;
                    changed = Some(app.clone());
                }
                if let Some(app) = changed {
                    crate::userdata::save(&app);
                }
                if self.selected == Some(id) {
                    self.selected = None;
                }
                self.set_status("App hidden from the list");
            }
            Act::Run(id) => {
                if let Some(app) = self.find(id) {
                    if let Some(exe) = &app.launch_exe {
                        crate::native::run_shell(&exe.to_string_lossy(), "", None, false);
                    } else if let Some(dir) = &app.install_dir {
                        crate::native::shell_open(&dir.to_string_lossy());
                    } else {
                        self.set_status("No executable found");
                    }
                }
            }
            Act::RegKey(id) => {
                if let Some(app) = self.find(id) {
                    if let Some((root, key)) = &app.reg_key {
                        let full = match root {
                            crate::model::RegRoot::Hklm => {
                                format!("HKEY_LOCAL_MACHINE\\{}", key)
                            }
                            crate::model::RegRoot::Hklm32 => {
                                format!("HKEY_LOCAL_MACHINE\\{}", key)
                            }
                            crate::model::RegRoot::Hkcu => {
                                format!("HKEY_CURRENT_USER\\{}", key)
                            }
                        };
                        // regedit takes no path argument; it reopens the key
                        // stored in LastKey, so write that first.
                        let _ = crate::native::run_cmd(
                            &format!(
                                "reg add \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Applets\\Regedit\" /v LastKey /t REG_SZ /d \"{}\" /f",
                                full
                            ),
                            true,
                        );
                        crate::native::run_shell("regedit.exe", "", None, false);
                        self.set_status("Registry opened");
                    } else {
                        self.set_status("This app has no registry key");
                    }
                }
            }
            Act::OpenStore(id) => {
                if let Some(app) = self.find(id) {
                    // The product page needs the family name; the search page
                    // works for every package, including side-loaded ones.
                    let q: String = app
                        .name
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '+' })
                        .collect();
                    crate::native::shell_open(&format!("ms-windows-store://search/?query={}", q));
                    self.set_status("Store opened");
                }
            }
        }
    }

    // ------------------------------------------------------------ drawing

    fn draw_header(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let p = self.palette.clone();
        let frame = Frame::NONE
            .fill(p.panel)
            .inner_margin(Margin::symmetric(18, 10));
        let _resp = egui::Panel::top("header")
            .frame(frame)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    {
                        // Brand mark
                        let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                        ui.painter().rect_filled(r, CornerRadius::same(8), p.accent);
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            "A",
                            FontId::proportional(theme::T_TITLE),
                            p.on_accent,
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("AppSweep")
                                .font(theme::bold(theme::T_HEADLINE))
                                .color(p.text),
                        );
                    }

                    // Search and navigation sit against the right edge.
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Added right to left, so the row reads: search, Scan,
                        // Settings, Backups.
                        if dialogs::nav_pill_icon(
                            ui,
                            &p,
                            glyph::Glyph::Backups,
                            "Backups",
                            self.show_backups,
                        ) {
                            self.show_backups = !self.show_backups;
                            if self.show_backups {
                                self.backups = crate::actions::list_backups();
                            }
                        }

                        if dialogs::nav_pill_icon(
                            ui,
                            &p,
                            glyph::Glyph::Settings,
                            "Settings",
                            self.show_settings,
                        ) {
                            self.show_settings = !self.show_settings;
                        }

                        if self.scanning {
                            ui.add(egui::Spinner::new().size(14.0));
                        } else if dialogs::nav_pill_icon(ui, &p, glyph::Glyph::Scan, "Scan", false)
                        {
                            self.start_scan(ctx);
                        }

                        // Right to left, so this space lands between the
                        // search pill and the Scan button: the frame's own edge
                        // is the visual boundary, and the default 8 pt gap read
                        // as if they touched.
                        ui.add_space(8.0);

                        // Search
                        let search_frame = Frame::NONE
                            .fill(if p.dark {
                                Color32::from_rgb(24, 24, 24)
                            } else {
                                Color32::from_rgb(240, 240, 240)
                            })
                            .stroke(Stroke::new(1.0, p.outline))
                            .corner_radius(CornerRadius::same(theme::R_PILL))
                            .inner_margin(Margin::symmetric(14, 0));
                        let mut focused = false;
                        let mut magnifier: Option<egui::Rect> = None;
                        let search = search_frame.show(ui, |ui| {
                            // Left to right inside the pill, whatever the bar does:
                            // the field first, then the clear button at the edge.
                            // The size is explicit because a bare layout would
                            // stretch the pill across the whole header.
                            let clear_w = if self.search.is_empty() { 0.0 } else { 26.0 };
                            ui.allocate_ui_with_layout(
                                vec2(244.0 + clear_w, theme::CONTROL_H),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.spacing_mut().interact_size.y = theme::CONTROL_H;
                                    let (ir, _) = ui.allocate_exact_size(
                                        vec2(18.0, 18.0),
                                        egui::Sense::hover(),
                                    );
                                    magnifier = Some(ir);
                                    ui.add_space(8.0);
                                    let edit = ui.add_sized(
                                        vec2(220.0, theme::CONTROL_H),
                                        egui::TextEdit::singleline(&mut self.search)
                                            .hint_text(
                                                RichText::new("Search or ID number")
                                                    .color(p.text_faint),
                                            )
                                            .frame(egui::Frame::NONE)
                                            .margin(Margin::ZERO)
                                            // A sized TextEdit lays its galley at
                                            // the top of the rect; centre it or the
                                            // hint sits high in the pill.
                                            .align(egui::Align2::LEFT_CENTER)
                                            .text_color(p.text),
                                    );
                                    focused = edit.has_focus();
                                    if !self.search.is_empty() {
                                        let (xr, clear) = ui.allocate_exact_size(
                                            vec2(18.0, 18.0),
                                            egui::Sense::click(),
                                        );
                                        if clear.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                        glyph::paint(ui, xr, glyph::Glyph::Close, p.text_dim);
                                        if clear.on_hover_text("Clear").clicked() {
                                            self.search.clear();
                                            edit.request_focus();
                                        }
                                    }
                                },
                            );
                        });
                        // The frame cannot draw its own focus state: it is built
                        // before the field knows it. Paint it over the response.
                        if focused {
                            ui.painter().rect_stroke(
                                search.response.rect,
                                CornerRadius::same(theme::R_PILL),
                                Stroke::new(1.6, p.accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if let Some(r) = magnifier {
                            glyph::paint(
                                ui,
                                r,
                                glyph::Glyph::Search,
                                if focused { p.accent } else { p.text_faint },
                            );
                        }
                    });
                });

                ui.add_space(6.0);

                // Filters
                ui.horizontal(|ui| {
                    for f in [
                        Filter::All,
                        Filter::Desktop,
                        Filter::Store,
                        Filter::System,
                        Filter::Favorites,
                        Filter::Remnants,
                    ] {
                        if dialogs::nav_pill(ui, &p, f.label(), self.filter == f) {
                            self.filter = f;
                        }
                    }
                });
            });
    }

    fn draw_status(&mut self, ui: &mut Ui) {
        let p = self.palette.clone();
        let frame = Frame::NONE
            .fill(p.panel)
            .inner_margin(Margin::symmetric(18, 7));
        egui::Panel::bottom("status")
            .frame(frame)
            .show_separator_line(false)
            .show(ui, |ui| {
                let total_w = ui.available_width();
                ui.horizontal(|ui| {
                    // Left block: view controls.
                    ui.allocate_ui_with_layout(
                        vec2((total_w - 460.0).max(120.0), theme::CONTROL_H),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            if dialogs::nav_pill_icon(
                                ui,
                                &p,
                                glyph::Glyph::Cards,
                                "Cards",
                                self.view == ViewMode::Cards,
                            ) {
                                self.view = ViewMode::Cards;
                                self.settings.view = ViewMode::Cards;
                            }
                            if dialogs::nav_pill_icon(
                                ui,
                                &p,
                                glyph::Glyph::Table,
                                "Table",
                                self.view == ViewMode::Table,
                            ) {
                                self.view = ViewMode::Table;
                                self.settings.view = ViewMode::Table;
                            }
                            ui.add_space(8.0);
                            if dialogs::nav_pill(
                                ui,
                                &p,
                                if self.settings.sort_desc {
                                    "Desc"
                                } else {
                                    "Asc"
                                },
                                false,
                            ) {
                                self.settings.sort_desc = !self.settings.sort_desc;
                            }
                            // No fixed limit: the menu opens at its natural
                            // height and only starts scrolling when it would run
                            // past the screen.
                            ui.spacing_mut().combo_height =
                                ui.ctx().viewport_rect().height() - 80.0;
                            egui::ComboBox::from_id_salt("sort-bottom")
                                .selected_text(self.settings.sort.label())
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for k in [
                                        SortKey::Name,
                                        SortKey::Publisher,
                                        SortKey::Size,
                                        SortKey::Date,
                                        SortKey::Source,
                                    ] {
                                        ui.selectable_value(&mut self.settings.sort, k, k.label());
                                    }
                                });
                            if !self.status.is_empty() {
                                let fresh = self
                                    .status_at
                                    .map(|t| t.elapsed().as_secs_f32() < 4.0)
                                    .unwrap_or(false);
                                if fresh {
                                    ui.add_space(10.0);
                                    ui.label(
                                        RichText::new(&self.status)
                                            .font(FontId::proportional(theme::T_BODY))
                                            .color(p.ok),
                                    );
                                }
                            }
                        },
                    );

                    // Right block: counters. Laid out right to left so it ends
                    // flush with the edge, which means the items are added in
                    // reverse reading order: Admin, warnings, shown, total, size.
                    ui.allocate_ui_with_layout(
                        vec2(440.0, theme::CONTROL_H),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            if self.elevated {
                                ui.label(
                                    RichText::new("Admin")
                                        .font(FontId::proportional(theme::T_LABEL))
                                        .color(p.ok),
                                );
                            }
                            if !self.warnings.is_empty() {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!("{}", self.warnings.len()))
                                        .font(FontId::proportional(theme::T_LABEL))
                                        .color(p.warn),
                                )
                                .on_hover_text(self.warnings.join("\n"));
                            }
                            let visible = self.filtered().len();
                            ui.label(
                                RichText::new(format!("{}", visible))
                                    .font(theme::bold(theme::T_BODY))
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new("Shown")
                                    .font(FontId::proportional(theme::T_BODY))
                                    .color(p.text_faint),
                            );
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(format!("{}", self.apps.len()))
                                    .font(theme::bold(theme::T_BODY))
                                    .color(p.text),
                            );
                            ui.label(
                                RichText::new("Total")
                                    .font(FontId::proportional(theme::T_BODY))
                                    .color(p.text_faint),
                            );
                            ui.add_space(10.0);
                            let bytes: u64 = self.apps.iter().map(|a| a.size_bytes).sum();
                            ui.label(
                                RichText::new(human_size(bytes))
                                    .font(FontId::proportional(theme::T_BODY))
                                    .color(p.text_dim),
                            );
                        },
                    );
                });
            });
    }

    fn draw_list(&mut self, ui: &mut Ui) {
        let p = self.palette.clone();
        let icon_px = cards::icon_px(self.settings.icon_size);
        let mut act: Option<Act> = None;
        let mut sort_click: Option<&'static str> = None;

        {
            let apps = self.filtered();
            if apps.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height() * 0.32).max(20.0));
                    if self.scanning {
                        ui.add(egui::Spinner::new().size(28.0));
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("Scanning installed apps")
                                .font(FontId::proportional(theme::T_UI))
                                .color(p.text_dim),
                        );
                    } else {
                        ui.label(
                            RichText::new(if self.scanned_once {
                                "No matching results"
                            } else {
                                "Not scanned yet"
                            })
                            .font(theme::bold(theme::T_TITLE))
                            .color(p.text_dim),
                        );
                        ui.add_space(8.0);
                        if widgets::button(ui, &p, "Scan now", widgets::Btn::Filled, 160.0, 36.0)
                            .clicked()
                        {
                            self.start_scan(ui.ctx());
                        }
                    }
                });
                return;
            }

            let need = widgets::NeedIcons::default();
            let c = Ctx {
                palette: &p,
                cache: &self.cache,
                elevated: self.elevated,
                need: &need,
                icon: self.settings.icon_size,
            };
            let selected = self.selected;
            let mut cols = 1usize;

            match self.view {
                ViewMode::Cards => {
                    let gap = self.settings.density.spacing() + 4.0;
                    cols = cards::layout(ui.available_width(), gap, self.settings.card_min_width).0;
                    let _out = egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(a) = cards::grid(
                                ui,
                                &c,
                                &apps,
                                selected,
                                icon_px,
                                gap,
                                self.settings.card_min_width,
                            ) {
                                act = Some(a);
                            }
                            ui.add_space(12.0);
                        });
                }
                ViewMode::Table => {
                    let width = ui.available_width();
                    let sort_label = self.settings.sort.label();
                    let hh = 30.0;
                    sort_click = table::header(ui, &c, width, hh, sort_label);
                    let rh = self.settings.density.row_height();
                    if let Some(a) =
                        table::body(ui, &c, &apps, selected, rh, width, self.settings.density)
                    {
                        act = Some(a);
                    }
                }
            }

            // The grid and the table are painted rects, so they can never take
            // focus themselves: the arrows act on the list whenever no text
            // field wants the keyboard.
            // shortcut: no scroll-into-view, so the selection can move past the
            // visible rows; upgrade when the list gets a scroll anchor.
            let modal_open = self.confirm.is_some()
                || self.details.is_some()
                || self.job.is_some()
                || self.show_settings
                || self.show_backups;
            if !modal_open && !ui.ctx().egui_wants_keyboard_input() && !apps.is_empty() {
                let len = apps.len();
                let cur = selected
                    .and_then(|id| apps.iter().position(|a| a.id == id))
                    .unwrap_or(0);
                let (up, down, left, right, home, end, enter, del) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::ArrowLeft),
                        i.key_pressed(egui::Key::ArrowRight),
                        i.key_pressed(egui::Key::Home),
                        i.key_pressed(egui::Key::End),
                        i.key_pressed(egui::Key::Enter),
                        i.key_pressed(egui::Key::Delete),
                    )
                });
                let step = cols as isize;
                let target = if up {
                    Some(nav_index(len, cur, -step))
                } else if down {
                    Some(nav_index(len, cur, step))
                } else if left {
                    Some(nav_index(len, cur, -1))
                } else if right {
                    Some(nav_index(len, cur, 1))
                } else if home {
                    Some(0)
                } else if end {
                    Some(len - 1)
                } else {
                    None
                };
                let cur = target.unwrap_or(cur);
                if enter {
                    act = Some(Act::Details(apps[cur].id));
                }
                if del {
                    act = Some(Act::Purge(apps[cur].id));
                }
                // Copied out first: pps borrows self, so it must die before
                // the selection is written back.
                if let Some(i) = target {
                    let id = apps[i].id;
                    self.selected = Some(id);
                }
            }

            let need_list = need.into_inner();
            for (id, size, path) in need_list {
                self.icons.request_raw(id, path, size);
            }
        }

        if let Some(k) = sort_click {
            let key = match k {
                "publisher" => SortKey::Publisher,
                "size" => SortKey::Size,
                "date" => SortKey::Date,
                "source" => SortKey::Source,
                _ => SortKey::Name,
            };
            if self.settings.sort == key {
                self.settings.sort_desc = !self.settings.sort_desc;
            } else {
                self.settings.sort = key;
                self.settings.sort_desc = false;
            }
        }

        if let Some(a) = act {
            self.handle(a, ui.ctx());
        }
    }

    fn draw_modals(&mut self, ctx: &egui::Context) {
        let p = self.palette.clone();

        if self.show_backups {
            let items = self.backups.clone();
            let mut confirm_clear = self.confirm_backup_clear;
            let (act, closed) = dialogs::show_modal(ctx, &p, "backups", |ui| {
                dialogs::backups(ui, &p, &items, &mut confirm_clear)
            });
            self.confirm_backup_clear = confirm_clear;
            match act {
                dialogs::BackupAct::Delete(path) => {
                    let entry = items
                        .iter()
                        .find(|b| b.path.to_string_lossy() == path)
                        .cloned();
                    if let Some(e) = entry {
                        match crate::actions::delete_backup(&e) {
                            Ok(()) => self.set_status("Backup deleted"),
                            Err(err) => {
                                self.set_status(format!("Could not delete the backup: {}", err))
                            }
                        }
                    }
                    self.backups = crate::actions::list_backups();
                }
                dialogs::BackupAct::ClearAll => {
                    match crate::actions::clear_backups() {
                        Ok(n) => {
                            self.set_status(format!("Deleted {} backups", n));
                        }
                        Err(err) => {
                            self.set_status(format!("Could not delete the backups: {}", err))
                        }
                    }
                    self.backups = crate::actions::list_backups();
                }
                dialogs::BackupAct::RevealFolder => {
                    let dir = crate::actions::backups_root();
                    let _ = std::fs::create_dir_all(&dir);
                    crate::native::open_in_explorer(&dir.to_string_lossy(), false);
                }
                dialogs::BackupAct::Import(path) => {
                    let (launch, code) =
                        crate::native::run_cmd(&format!("reg import \"{}\"", path), true);
                    if launch == 0 && code == 0 {
                        self.set_status("Backup imported into the registry");
                    } else {
                        let bad = if launch != 0 { launch } else { code as i32 };
                        self.set_status(format!("Import failed, code {}", bad));
                    }
                }
                dialogs::BackupAct::None => {}
            }
            if closed {
                self.show_backups = false;
            }
        }

        if self.show_settings {
            let mut s = self.settings.clone();
            let (asked, changed) = {
                let (inner, closed) = dialogs::show_modal(ctx, &p, "settings", |ui| {
                    dialogs::settings(ui, &p, &mut s)
                });
                if closed {
                    (Asked::Close, true)
                } else {
                    inner
                }
            };
            if changed {
                let theme_changed = s.theme != self.settings.theme
                    || s.accent != self.settings.accent
                    || s.density != self.settings.density;
                self.settings = s.clone();
                self.view = s.view;
                if theme_changed {
                    self.palette = Palette::new(s.theme, s.accent);
                    theme::apply(ctx, s.theme, s.accent, s.density);
                }
                self.settings.save();
            }
            if matches!(asked, Asked::Close) {
                self.show_settings = false;
                self.settings.save();
            }
        }

        if let Some(state) = &mut self.confirm {
            let (asked, closed) =
                dialogs::show_modal(ctx, &p, "confirm", |ui| dialogs::confirm(ui, &p, state));
            let asked = if closed { Asked::Close } else { asked };
            match asked {
                Asked::Close => self.confirm = None,
                Asked::Confirm {
                    app_id,
                    flow,
                    auto_cascade,
                    keep_data,
                } => {
                    let mut s = self.settings.clone();
                    s.auto_cascade = auto_cascade;
                    if let Some(app) = self.find(app_id) {
                        let app = app.clone();
                        self.job = Some(actions::spawn(app, flow, s, keep_data));
                    }
                    self.confirm = None;
                }
                _ => {}
            }
        }

        if let Some(job) = &mut self.job {
            job.poll();
            let (asked, closed) =
                dialogs::show_modal(ctx, &p, "job", |ui| dialogs::job(ui, &p, job));
            let asked = if closed && job.done {
                Asked::Close
            } else {
                asked
            };
            match asked {
                Asked::Close => {
                    // Read everything off `job` before `self.job` is cleared.
                    let success = job.done && job.success;
                    let failed = job.steps.iter().any(|s| s.status == StepStatus::Fail);
                    self.job = None;
                    self.cache.clear();
                    if success {
                        self.set_status("Operation completed successfully");
                    } else if failed {
                        self.set_status("Operation failed - see the steps");
                    } else {
                        self.set_status("Finished with warnings");
                    }
                    self.start_scan(ctx);
                }
                Asked::Rescan => {
                    self.job = None;
                    self.cache.clear();
                    self.start_scan(ctx);
                }
                _ => {}
            }
        }

        if let Some(id) = self.details {
            let app = self.find(id).cloned();
            match app {
                Some(app) => {
                    let need = widgets::NeedIcons::default();
                    let c = Ctx {
                        palette: &p,
                        cache: &self.cache,
                        elevated: self.elevated,
                        need: &need,
                        icon: self.settings.icon_size,
                    };
                    let (asked, closed) = dialogs::show_modal(ctx, &p, "details", |ui| {
                        dialogs::details(ui, &p, &app, &c)
                    });
                    let asked = if closed { Asked::Close } else { asked };
                    let pending = need.into_inner();
                    for (id, size, path) in pending {
                        self.icons.request_raw(id, path, size);
                    }
                    match asked {
                        Asked::Close => self.details = None,
                        Asked::OpenFolder(id) => {
                            self.details = None;
                            self.handle(Act::Reveal(id), ctx);
                        }
                        Asked::CopyPath(id) => self.handle(Act::CopyPath(id), ctx),
                        Asked::Run(id) => self.handle(Act::Run(id), ctx),
                        Asked::SetNotes { app_id, notes } => {
                            let mut changed = None;
                            if let Some(a) = self.apps.iter_mut().find(|a| a.id == app_id) {
                                a.notes = notes;
                                changed = Some(a.clone());
                            }
                            if let Some(a) = changed {
                                crate::userdata::save(&a);
                            }
                        }
                        Asked::Confirm {
                            app_id,
                            flow,
                            auto_cascade,
                            keep_data,
                        } => {
                            self.details = None;
                            if let Some(app) = self.find(app_id) {
                                self.confirm = Some(ConfirmState {
                                    app: app.clone(),
                                    flow,
                                    stage: 1,
                                    ack: false,
                                    auto_cascade,
                                    keep_data,
                                    typed: String::new(),
                                });
                            }
                        }
                        Asked::Rescan => {
                            self.details = None;
                            self.start_scan(ctx);
                        }
                        Asked::None => {}
                    }
                }
                None => self.details = None,
            }
        }
    }
}

impl eframe::App for AppSweep {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll the scanner.
        let mut done = None;
        if let Some(rx) = &self.scan_rx {
            if let Ok(res) = rx.try_recv() {
                done = Some(res);
            }
        }
        if let Some(res) = done {
            self.apps = res.apps;
            crate::userdata::apply(&mut self.apps);
            self.warnings = res.warnings;
            self.scanning = false;
            self.scan_rx = None;
            self.scanned_once = true;
            self.scan_at = Some(Instant::now());
            self.cache.clear();
            self.set_status("Scan finished");
            self.start_enrich(ctx);
            self.debug_open();
        }

        // Logo/size batches streaming in from the enrich threads.
        if let Some(rx) = &self.enrich_rx {
            let mut batches = Vec::new();
            let mut closed = false;
            loop {
                match rx.try_recv() {
                    Ok(b) => batches.push(b),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        closed = true;
                        break;
                    }
                }
            }
            for b in batches {
                self.apply_enrich(b);
            }
            if closed {
                self.enrich_rx = None;
                self.enriching = false;
            }
        }

        // Icons that finished extracting.
        for d in self.icons.drain() {
            let (id, size) = parse_key(&d.key);
            self.cache.insert(id, size, d.image, ctx);
        }

        if self.scanning || self.job.is_some() || self.enriching {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        if self
            .status_at
            .map(|t| t.elapsed().as_secs_f32() < 4.5)
            .unwrap_or(false)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Behind every panel: the same page colour, so a resize never flashes.
        self.palette.bg.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.palette = Palette::new(self.settings.theme, self.settings.accent);
        let p = self.palette.clone();
        ui.style_mut().visuals.panel_fill = p.bg;
        let ctx = ui.ctx().clone();

        self.draw_header(ui, &ctx);
        self.draw_status(ui);

        // `no_frame` means no fill either, so the window's clear colour showed
        // through every margin and card gap. Fill it with the page colour.
        egui::CentralPanel::no_frame()
            .frame(Frame::NONE.fill(p.bg))
            .show(ui, |ui| {
                let margin = Frame::NONE.inner_margin(Margin::symmetric(18, 12));
                margin.show(ui, |ui| {
                    self.draw_list(ui);
                });
            });

        self.draw_modals(&ctx);
    }
}

fn parse_key(key: &str) -> (u64, u32) {
    let mut it = key.split('|');
    let id = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let size = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (id, size)
}

/// Keeps the compiler honest about the atomic flag we no longer use.
#[allow(dead_code)]
fn _unused_atomic(f: &Arc<AtomicBool>) {
    let _ = f.load(Ordering::Relaxed);
}

/// Where an arrow key lands: `cur` moved by `delta`, clamped to the list.
fn nav_index(len: usize, cur: usize, delta: isize) -> usize {
    (cur as isize + delta).clamp(0, len as isize - 1) as usize
}

#[cfg(test)]
mod tests {
    use super::nav_index;

    #[test]
    fn arrows_stay_inside_the_list() {
        // Both edges must clamp instead of wrapping to the other end.
        assert_eq!(nav_index(10, 0, -1), 0);
        assert_eq!(nav_index(10, 9, 1), 9);
        assert_eq!(nav_index(10, 4, 3), 7);
        assert_eq!(nav_index(10, 8, 3), 9);
        // A grid step larger than the list still has to land on the last row.
        assert_eq!(nav_index(10, 9, -4), 5);
        assert_eq!(nav_index(1, 0, -3), 0);
    }
}
