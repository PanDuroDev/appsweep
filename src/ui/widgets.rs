//! Small hand-painted widgets used by the cards, the table and the dialogs.
//!
//! Everything here is painted with the egui painter so the layout stays under
//! our control. Note that epaint 0.37 has no full bidi support: a string that
//! mixes Arabic with Latin letters or digits is laid out in the wrong order, so
//! Arabic and non-Arabic text is always split into separate runs.

use crate::icons::IconCache;
use crate::model::AppEntry;
use crate::theme::Palette;
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke,
    StrokeKind, Ui,
};

/// Icon requests collected while painting, so the app can queue them after the
/// frame without holding a mutable borrow of the cache.
pub type NeedIcons = std::cell::RefCell<Vec<(u64, u32, String)>>;

pub struct Ctx<'a> {
    pub palette: &'a Palette,
    pub cache: &'a IconCache,
    pub elevated: bool,
    pub need: &'a NeedIcons,
    pub icon: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    Select(u64),
    Details(u64),
    Uninstall(u64),
    Purge(u64),
    Files(u64),
    Reveal(u64),
    Props(u64),
    CopyPath(u64),
    Favorite(u64),
    Hide(u64),
    Run(u64),
    RegKey(u64),
    OpenStore(u64),
}

pub fn icon_texture<'a>(
    cache: &'a IconCache,
    app: &AppEntry,
    size: u32,
) -> Option<&'a egui::TextureHandle> {
    cache.get(app, size)
}

/// True for characters that belong to an Arabic run.
pub fn is_arabic(c: char) -> bool {
    matches!(c as u32,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF |
        0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

/// Split a mixed string into alternating runs of (is_arabic, text).
pub fn bidi_runs(text: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    for c in text.chars() {
        let arabic = is_arabic(c);
        match out.last_mut() {
            Some((flag, buf)) if *flag == arabic => buf.push(c),
            Some((flag, buf)) if c.is_whitespace() => {
                let _ = flag;
                buf.push(c);
            }
            _ => out.push((arabic, c.to_string())),
        }
    }
    out
}

/// Paint a mixed Arabic/Latin string in the correct visual order.
///
/// Runs are placed right-to-left inside `rect`, each run itself drawn left to
/// right, which is what a reader of a right-to-left line expects.
pub fn bidi_line(ui: &Ui, rect: Rect, text: &str, font: FontId, color: Color32) {
    let runs = bidi_runs(text);
    if runs.is_empty() {
        return;
    }
    let mut x = rect.right();
    let cy = rect.center().y;
    for (_, run) in runs.iter().rev() {
        let galley = ui
            .painter()
            .layout_no_wrap(run.clone(), font.clone(), color);
        let w = galley.size().x;
        if x - w < rect.left() {
            // Out of room: paint what still fits and stop.
            let avail = (x - rect.left()).max(0.0);
            if avail > 4.0 {
                ui.painter()
                    .galley(pos2(x - w, cy - galley.size().y * 0.5), galley, color);
            }
            break;
        }
        ui.painter()
            .galley(pos2(x - w, cy - galley.size().y * 0.5), galley, color);
        x -= w;
    }
}

/// Single-line text with an ellipsis when it does not fit.
pub fn text_fit(ui: &Ui, rect: Rect, text: &str, font: FontId, color: Color32, align: Align2) {
    if rect.width() < 4.0 {
        return;
    }
    let mut job = LayoutJob::single_section(
        text.to_string(),
        TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    job.wrap = TextWrapping {
        max_width: rect.width(),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job.halign = Align::LEFT;
    let galley = ui.painter().layout_job(job);
    let x = match align {
        Align2::RIGHT_CENTER => rect.right() - galley.size().x,
        Align2::LEFT_CENTER => rect.left(),
        _ => rect.center().x - galley.size().x * 0.5,
    };
    ui.painter().galley(
        pos2(x, rect.center().y - galley.size().y * 0.5),
        galley,
        color,
    );
}

/// A rounded pill drawn straight onto the painter, returns the click response.
#[allow(clippy::too_many_arguments)]
pub fn pill(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    rect: Rect,
    label: &str,
    fg: Color32,
    bg: Color32,
    bg_hover: Color32,
    font_size: f32,
) -> Response {
    let id = ui.auto_id_with(salt);
    let resp = ui.interact(rect, id, Sense::click());
    let hovered = resp.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(7),
        if hovered { bg_hover } else { bg },
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(font_size),
        fg,
    );
    resp
}

/// Star toggle used for favourites.
pub fn star(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    rect: Rect,
    active: bool,
    p: &Palette,
) -> Response {
    let id = ui.auto_id_with(salt);
    let resp = ui.interact(rect, id, Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let color = if active {
        p.warn
    } else if resp.hovered() {
        p.text_dim
    } else {
        p.text_faint
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        if active { "★" } else { "☆" },
        FontId::proportional(15.0),
        color,
    );
    resp
}

pub fn chip(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(11.0), fg);
    let size = galley.size() + vec2(14.0, 6.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, fg);
}

/// Draw a chip whose right edge is at `right`, returns the rect it used.
#[allow(clippy::too_many_arguments)]
pub fn chip_right(
    ui: &Ui,
    right: f32,
    top: f32,
    text: &str,
    fg: Color32,
    bg: Color32,
    font_size: f32,
    max_w: f32,
) -> Rect {
    let mut job = LayoutJob::single_section(
        text.to_string(),
        TextFormat {
            font_id: FontId::proportional(font_size),
            color: fg,
            ..Default::default()
        },
    );
    job.wrap = TextWrapping {
        max_width: (max_w - 12.0).max(10.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.painter().layout_job(job);
    let w = galley.size().x + 12.0;
    let h = galley.size().y + 6.0;
    let rect = Rect::from_min_size(pos2(right - w, top), vec2(w, h));
    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, fg);
    rect
}

pub fn paint_icon(ui: &mut Ui, ctx: &Ctx, app: &AppEntry, rect: Rect, size: u32) {
    let p = ctx.palette;
    if !ctx.cache.contains(app, size) {
        if let Some(src) = app.icon_source() {
            let mut need = ctx.need.borrow_mut();
            if !need.iter().any(|(id, s, _)| *id == app.id && *s == size) {
                need.push((app.id, size, src));
            }
        }
    }
    match icon_texture(ctx.cache, app, size) {
        Some(tex) => {
            let uv = Rect::from_min_max(pos2(0.0, 1.0), pos2(1.0, 0.0));
            ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
        }
        None => {
            // Placeholder: rounded square with the first letter of the name.
            ui.painter()
                .rect_filled(rect, CornerRadius::same(9), p.accent_soft);
            let letter = app
                .name
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "?".to_string());
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                letter,
                FontId::proportional(rect.height() * 0.45),
                p.accent_text,
            );
        }
    }
}

/// A small coloured dot used for statuses.
pub fn dot_at(ui: &Ui, center: egui::Pos2, color: Color32) {
    ui.painter().circle_filled(center, 4.0, color);
}

pub fn card_rect_style(ui: &mut Ui, ctx: &Ctx, w: f32, h: f32, selected: bool) -> (Rect, Response) {
    let p = ctx.palette;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let hovered = resp.hovered();
    let fill = if selected {
        p.accent_soft
    } else if hovered {
        p.card_hover
    } else {
        p.card
    };
    let stroke = if selected {
        Stroke::new(1.6, p.accent)
    } else if hovered {
        Stroke::new(1.0, p.border_strong)
    } else {
        Stroke::new(1.0, p.border)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(12),
        fill,
        stroke,
        StrokeKind::Inside,
    );
    (rect, resp)
}
