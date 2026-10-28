//! Table view: a hand-rolled virtualised list so thousands of rows stay smooth.

use super::widgets::{self, Act, Ctx};
use crate::model::{human_size, AppEntry};
use crate::settings::Density;
use crate::theme;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui,
};

/// Column widths, measured from the left edge.
struct Cols {
    name_w: f32,
    publisher_w: f32,
    version_w: f32,
    size_w: f32,
    date_w: f32,
    source_w: f32,
    actions_w: f32,
}

impl Cols {
    fn for_width(w: f32) -> Self {
        let actions_w = 196.0;
        let source_w = 74.0;
        let date_w = 88.0;
        let size_w = 70.0;
        let version_w = 82.0;
        let publisher_w = if w > 1180.0 { 170.0 } else { 0.0 };
        let used = actions_w + source_w + date_w + size_w + version_w + publisher_w;
        let name_w = (w - used).max(140.0);
        Self {
            name_w,
            publisher_w,
            version_w,
            size_w,
            date_w,
            source_w,
            actions_w,
        }
    }
}

pub fn row_height(d: Density) -> f32 {
    match d {
        Density::Compact => 32.0,
        Density::Normal => 42.0,
        Density::Relaxed => 54.0,
    }
}

/// Draw the sticky header, returns nothing but consumes vertical space.
pub fn header(
    ui: &mut Ui,
    ctx: &Ctx,
    width: f32,
    h: f32,
    sort_label: &str,
) -> Option<&'static str> {
    let p = ctx.palette;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    ui.painter().rect_filled(rect, 0, p.bg_alt);
    ui.painter().rect_stroke(
        Rect::from_min_max(pos2(rect.left(), rect.bottom() - 1.0), rect.max),
        0,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    let c = Cols::for_width(width);
    let font = FontId::proportional(theme::T_BODY);
    let color = p.text_faint;
    let _cy = rect.center().y;

    let mut x = rect.left();
    let mut col = |ui: &Ui, w: f32, label: &str| {
        if w <= 0.0 {
            x += w;
            return;
        }
        let r = Rect::from_min_max(pos2(x, rect.top()), pos2(x + w, rect.bottom()));
        if !label.is_empty() {
            widgets::bidi_line(ui, r.shrink2(vec2(10.0, 0.0)), label, font.clone(), color);
        }
        x += w;
    };

    col(ui, c.name_w, sort_label);
    col(ui, c.publisher_w, "Publisher");
    col(ui, c.version_w, "Version");
    col(ui, c.size_w, "Size");
    col(ui, c.date_w, "Installed");
    col(ui, c.source_w, "Source");
    col(ui, c.actions_w, "Actions");

    // Clickable zones for sorting.
    let mut zone_x = rect.left();
    let mut hit: Option<&'static str> = None;
    let mut zone = |ui: &mut Ui, salt: &'static str, w: f32| {
        if w <= 0.0 {
            zone_x += w;
            return;
        }
        let r = Rect::from_min_max(pos2(zone_x, rect.top()), pos2(zone_x + w, rect.bottom()));
        let resp = ui.interact(r, ui.auto_id_with(("sort", salt)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            ui.painter()
                .rect_filled(r.shrink2(vec2(2.0, 3.0)), 5, p.card_hover);
        }
        if resp.clicked() {
            hit = Some(salt);
        }
        zone_x += w;
    };
    zone(ui, "name", c.name_w);
    zone(ui, "publisher", c.publisher_w);
    zone(ui, "version", c.version_w);
    zone(ui, "size", c.size_w);
    zone(ui, "date", c.date_w);
    zone(ui, "source", c.source_w);
    hit
}

/// Draw one table row. Returns the action requested, if any.
pub fn row(
    ui: &mut Ui,
    ctx: &Ctx,
    app: &AppEntry,
    width: f32,
    h: f32,
    selected: bool,
    index: usize,
) -> Option<Act> {
    let p = ctx.palette;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, h), Sense::click());
    if !ui.is_rect_visible(rect) {
        return None;
    }
    let mut act = None;

    let bg = if selected {
        p.accent_soft
    } else if resp.hovered() {
        p.card_hover
    } else if index % 2 == 1 {
        p.bg_alt
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0, bg);
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + 2.0)),
            0,
            p.accent,
        );
    }
    ui.painter().rect_stroke(
        Rect::from_min_max(pos2(rect.left(), rect.bottom() - 1.0), rect.max),
        0,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    if resp.clicked() {
        act = Some(Act::Select(app.id));
    }
    if resp.double_clicked() {
        act = Some(Act::Run(app.id));
    }

    let c = Cols::for_width(width);
    let cy = rect.center().y;
    let mut x = rect.left();

    // name column, with icon
    {
        let r = Rect::from_min_max(pos2(x, rect.top()), pos2(x + c.name_w, rect.bottom()));
        let inner = r.shrink2(vec2(10.0, 0.0));
        let isz = (h - 14.0).clamp(16.0, 28.0);
        let icon_rect = Rect::from_min_size(pos2(inner.left(), cy - isz * 0.5), vec2(isz, isz));
        widgets::paint_icon(ui, ctx, app, icon_rect, (isz * 1.5) as u32);
        // Clicking the icon opens the full details sheet.
        if ui
            .interact(
                icon_rect,
                ui.auto_id_with(("rowicon", app.id)),
                Sense::click(),
            )
            .on_hover_text("Show details")
            .clicked()
        {
            act = Some(Act::Details(app.id));
        }
        let star_rect =
            Rect::from_min_size(pos2(icon_rect.right() + 4.0, cy - 11.0), vec2(22.0, 22.0));
        if widgets::star(ui, ("tstar", app.id), star_rect, app.favorite, p).clicked() {
            act = Some(Act::Favorite(app.id));
        }
        let remnant = app.is_remnant();
        // Numeric id badge, placed just right of the star so it reads as part of
        // the name cell. This is the id the permanent-delete gate asks for.
        let id_text = format!("#{}", app.id);
        let id_w = 34.0;
        let id_r = Rect::from_min_max(
            pos2(star_rect.right() + 6.0, cy - 9.0),
            pos2(star_rect.right() + 6.0 + id_w, cy + 9.0),
        );
        ui.painter().rect_filled(
            id_r,
            CornerRadius::same(theme::R_XS),
            if p.dark {
                Color32::from_rgb(26, 32, 43)
            } else {
                Color32::from_rgb(240, 243, 248)
            },
        );
        ui.painter().text(
            id_r.center(),
            Align2::CENTER_CENTER,
            id_text,
            FontId::monospace(9.5),
            p.text_faint,
        );
        let text_r = Rect::from_min_max(
            pos2(id_r.right() + 6.0, cy - 10.0),
            pos2(inner.right(), cy + 10.0),
        );
        widgets::text_fit(
            ui,
            text_r,
            &app.name,
            theme::bold(theme::T_UI),
            p.text,
            Align2::LEFT_CENTER,
        );
        if app.is_system {
            let dot_c = pos2(text_r.right() - 3.0, cy);
            widgets::dot_at(ui, dot_c, p.warn);
        }
        if remnant {
            // Nothing left on disk: the registry entry is a leftover.
            let name_w = ui
                .painter()
                .layout_no_wrap(app.name.clone(), theme::bold(theme::T_UI), p.text)
                .size()
                .x;
            let chip_left = text_r.left() + name_w + 8.0;
            widgets::chip_left(
                ui,
                chip_left,
                cy - 8.0,
                "Remnant",
                p.danger,
                p.danger_soft,
                10.0,
                70.0,
            );
        }
        x += c.name_w;
    }

    let mut cell = |ui: &Ui, w: f32, text: &str, font: FontId, color: Color32| {
        if w <= 0.0 {
            x += w;
            return;
        }
        let r = Rect::from_min_max(pos2(x, rect.top()), pos2(x + w, rect.bottom()));
        if !text.is_empty() {
            widgets::text_fit(
                ui,
                r.shrink2(vec2(10.0, 0.0)),
                text,
                font.clone(),
                color,
                Align2::LEFT_CENTER,
            );
        }
        x += w;
    };

    cell(
        ui,
        c.publisher_w,
        &app.publisher,
        FontId::proportional(theme::T_BODY),
        p.text_dim,
    );
    cell(
        ui,
        c.version_w,
        &app.version,
        FontId::proportional(theme::T_BODY),
        p.text_dim,
    );
    cell(
        ui,
        c.size_w,
        &human_size(app.size_bytes),
        FontId::proportional(theme::T_BODY),
        p.text_dim,
    );
    cell(
        ui,
        c.date_w,
        &app.install_date,
        FontId::proportional(theme::T_BODY),
        p.text_faint,
    );

    // source chip
    {
        let r = Rect::from_min_max(pos2(x, rect.top()), pos2(x + c.source_w, rect.bottom()));
        let (fg, bg) = if app.is_system {
            (p.warn, p.warn_soft)
        } else if app.source == crate::model::Source::Uwp {
            (p.accent_text, p.accent_soft)
        } else {
            (p.text_dim, p.card_hover)
        };
        let galley = ui.painter().layout_no_wrap(
            app.source.short().to_string(),
            FontId::proportional(theme::T_LABEL),
            fg,
        );
        let w = galley.size().x + 12.0;
        let chip = Rect::from_center_size(
            pos2(r.left() + 10.0 + w * 0.5, cy),
            vec2(w, galley.size().y + 6.0),
        );
        ui.painter().rect_filled(chip, 6, bg);
        ui.painter()
            .galley(chip.center() - galley.size() * 0.5, galley, fg);
        x += c.source_w;
    }

    // action buttons
    {
        let r = Rect::from_min_max(pos2(x, rect.top()), pos2(x + c.actions_w, rect.bottom()));
        let bh = (h - 12.0).clamp(20.0, 28.0);
        let by = cy - bh * 0.5;
        let mut bx = r.left() + 8.0;

        let folder = Rect::from_min_size(pos2(bx, by), vec2(bh, bh));
        if widgets::pill(
            ui,
            ("trow-reveal", app.id),
            folder,
            "⌸",
            p.text_dim,
            p.card,
            p.card_hover,
            14.0,
        )
        .on_hover_text("Open the app folder")
        .clicked()
        {
            act = Some(Act::Reveal(app.id));
        }
        bx += bh + 5.0;

        let blocked = app.blocked_reason();
        let un_w = 54.0;
        if widgets::pill(
            ui,
            ("trow-un", app.id),
            Rect::from_min_size(pos2(bx, by), vec2(un_w, bh)),
            "Uninstall",
            if blocked.is_some() {
                p.text_faint
            } else {
                p.accent_text
            },
            if blocked.is_some() {
                p.card
            } else {
                p.accent_soft
            },
            if blocked.is_some() {
                p.card
            } else {
                Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 70)
            },
            11.0,
        )
        .on_hover_text(blocked.unwrap_or("Run the uninstaller with automatic fallbacks"))
        .clicked()
        {
            act = Some(Act::Uninstall(app.id));
        }
        bx += un_w + 5.0;

        let pu_w = 66.0;
        if widgets::pill(
            ui,
            ("trow-pu", app.id),
            Rect::from_min_size(pos2(bx, by), vec2(pu_w, bh)),
            "Purge",
            if blocked.is_some() {
                p.text_faint
            } else {
                p.danger
            },
            if blocked.is_some() {
                p.card
            } else {
                p.danger_soft
            },
            if blocked.is_some() {
                p.card
            } else {
                Color32::from_rgba_unmultiplied(240, 100, 112, 70)
            },
            11.0,
        )
        .on_hover_text(blocked.unwrap_or("Permanent deletion of every root and leftover"))
        .clicked()
        {
            act = Some(Act::Purge(app.id));
        }

        let id = app.id;
        resp.context_menu(|ui| {
            if ui.button("Launch app").clicked() {
                act = Some(Act::Run(id));
                ui.close();
            }
            if ui.button("Show details").clicked() {
                act = Some(Act::Details(id));
                ui.close();
            }
            if ui.button("Delete files only").clicked() {
                act = Some(Act::Files(id));
                ui.close();
            }
            if ui.button("File properties").clicked() {
                act = Some(Act::Props(id));
                ui.close();
            }
            if ui.button("Copy path").clicked() {
                act = Some(Act::CopyPath(id));
                ui.close();
            }
            if app.reg_key.is_some() && ui.button("Open registry key").clicked() {
                act = Some(Act::RegKey(id));
                ui.close();
            }
            if app.source == crate::model::Source::Uwp && ui.button("Open in Store").clicked() {
                act = Some(Act::OpenStore(id));
                ui.close();
            }
            ui.separator();
            if ui.button("Hide from list").clicked() {
                act = Some(Act::Hide(id));
                ui.close();
            }
        });
    }

    act
}

/// Virtualised table body.
pub fn body(
    ui: &mut Ui,
    ctx: &Ctx,
    apps: &[&AppEntry],
    selected: Option<u64>,
    _h: f32,
    width: f32,
    density: Density,
) -> Option<Act> {
    let mut act = None;
    let rh = row_height(density);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, rh, apps.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for i in range {
                if let Some(app) = apps.get(i) {
                    if let Some(a) = row(ui, ctx, app, width, rh, selected == Some(app.id), i) {
                        act = Some(a);
                    }
                }
            }
        });
    act
}
