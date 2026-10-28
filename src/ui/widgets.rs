//! Small hand-painted widgets used by the cards, the table and the dialogs.
//!
//! Everything here is painted with the egui painter so the layout stays under
//! our control. Note that epaint 0.37 has no full bidi support: a string that
//! mixes Arabic with Latin letters or digits is laid out in the wrong order, so
//! Arabic and non-Arabic text is always split into separate runs.

use crate::icons::IconCache;
use crate::model::AppEntry;
use crate::theme::{Palette, R_CARD, R_PILL, R_SM, R_XS, T_LABEL, T_TITLE, T_UI};
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
/// The direction comes from the first non-space run, so an English sentence
/// with a Latin path in it stays left-aligned, while an Arabic name or path
/// still reads right to left.
pub fn bidi_line(ui: &Ui, rect: Rect, text: &str, font: FontId, color: Color32) {
    let runs = bidi_runs(text);
    if runs.is_empty() {
        return;
    }
    let rtl = runs
        .iter()
        .find(|(_, run)| !run.trim().is_empty())
        .map(|(arabic, _)| *arabic)
        .unwrap_or(false);
    let cy = rect.center().y;
    let mut x = if rtl { rect.right() } else { rect.left() };
    let ordered: Vec<&(bool, String)> = if rtl {
        runs.iter().rev().collect()
    } else {
        runs.iter().collect()
    };
    for (_, run) in ordered {
        let galley = ui
            .painter()
            .layout_no_wrap(run.clone(), font.clone(), color);
        let w = galley.size().x;
        let start = if rtl { x - w } else { x };
        if start < rect.left() || start + w > rect.right() {
            // Out of room: paint what still fits and stop.
            let avail = if rtl {
                x - rect.left()
            } else {
                rect.right() - x
            };
            if avail > 4.0 {
                ui.painter()
                    .galley(pos2(start, cy - galley.size().y * 0.5), galley, color);
            }
            break;
        }
        ui.painter()
            .galley(pos2(start, cy - galley.size().y * 0.5), galley, color);
        if rtl {
            x -= w;
        } else {
            x += w;
        }
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

/// Composite an alpha state layer over an opaque fill (the base is opaque by
/// construction: pills, cards and rows all sit on a solid colour).
pub fn tint(base: Color32, layer: Color32) -> Color32 {
    // epaint colours are premultiplied, so the layer's rgb is already scaled by
    // its alpha and only the base needs weighting.
    let a = layer.a() as u32;
    let mix = |b: u8, l: u8| (l as u32 + b as u32 * (255 - a) / 255).min(255) as u8;
    Color32::from_rgb(
        mix(base.r(), layer.r()),
        mix(base.g(), layer.g()),
        mix(base.b(), layer.b()),
    )
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
    let fill = if resp.is_pointer_button_down_on() {
        // Pressed: one tonal step past hover, so a click feels answered.
        let layer = if ui.visuals().dark_mode {
            Color32::from_rgba_unmultiplied(255, 255, 255, 31)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 31)
        };
        tint(if hovered { bg_hover } else { bg }, layer)
    } else if hovered {
        bg_hover
    } else {
        bg
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(R_PILL), fill);
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
        FontId::proportional(T_TITLE),
        color,
    );
    resp
}

pub fn chip(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(T_LABEL), fg);
    let size = galley.size() + vec2(14.0, 6.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(R_XS), bg);
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, fg);
}

/// A pill with a glyph in front of its label, centred as one unit.
#[allow(clippy::too_many_arguments)]
pub fn icon_pill(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    rect: Rect,
    glyph: super::glyph::Glyph,
    label: &str,
    fg: Color32,
    bg: Color32,
    bg_hover: Color32,
    font_size: f32,
) -> Response {
    let resp = pill(ui, salt, rect, "", fg, bg, bg_hover, font_size);
    let icon = font_size + 2.0;
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), FontId::proportional(font_size), fg);
    let gap = if label.is_empty() { 0.0 } else { 6.0 };
    let total = icon + gap + galley.size().x;
    let left = rect.center().x - total * 0.5;
    super::glyph::paint_at(
        ui,
        pos2(left + icon * 0.5, rect.center().y),
        icon,
        glyph,
        fg,
    );
    if !label.is_empty() {
        ui.painter().galley(
            pos2(left + icon + gap, rect.center().y - galley.size().y * 0.5),
            galley,
            fg,
        );
    }
    resp
}

/// Draw a chip whose left edge is at `left`, returns the rect it used.
#[allow(clippy::too_many_arguments)]
pub fn chip_left(
    ui: &Ui,
    left: f32,
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
    let rect = Rect::from_min_size(pos2(left, top), vec2(w, h));
    ui.painter().rect_filled(rect, CornerRadius::same(R_XS), bg);
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, fg);
    rect
}

/// Material button role.
#[derive(Clone, Copy)]
pub enum Btn {
    Filled,
    Text,
    DangerTonal,
}

/// A button with a Material state layer: the fill stays put and a translucent
/// overlay marks hover and press, which .fill() on its own would suppress.
pub fn button(ui: &mut Ui, p: &Palette, label: &str, kind: Btn, w: f32, h: f32) -> Response {
    let (fg, fill, stroke) = match kind {
        Btn::Filled => (p.on_accent, p.accent, Stroke::NONE),
        Btn::Text => (p.text_dim, Color32::TRANSPARENT, Stroke::NONE),
        Btn::DangerTonal => (p.danger, p.danger_soft, Stroke::NONE),
    };
    button_colored(ui, p, label, fg, fill, stroke, w, h)
}

/// Same as [utton] but with colours the caller picks, for the few buttons
/// whose state is expressed by colour alone (disabled, launch, backup rows).
#[allow(clippy::too_many_arguments)]
pub fn button_colored(
    ui: &mut Ui,
    p: &Palette,
    label: &str,
    fg: Color32,
    fill: Color32,
    stroke: Stroke,
    w: f32,
    h: f32,
) -> Response {
    // The label is painted by hand so the state layer can go underneath it;
    // a Button that draws its own text would tint the text, not the fill.
    let resp = ui.add(
        egui::Button::new("")
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(R_PILL))
            .min_size(vec2(w, h)),
    );
    let layer = if resp.is_pointer_button_down_on() {
        p.press_layer
    } else if resp.hovered() {
        p.hover_layer
    } else {
        Color32::TRANSPARENT
    };
    if layer.a() > 0 {
        ui.painter()
            .rect_filled(resp.rect, CornerRadius::same(R_PILL), layer);
    }
    ui.painter().text(
        resp.rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(T_UI),
        fg,
    );
    resp
}

/// Segmented-control option: outlined when idle, tinted when selected.
pub fn segment(ui: &mut Ui, p: &Palette, label: &str, selected: bool, w: f32) -> Response {
    button_colored(
        ui,
        p,
        label,
        if selected { p.accent_text } else { p.text_dim },
        if selected {
            p.accent_soft
        } else {
            Color32::TRANSPARENT
        },
        Stroke::new(1.0, if selected { p.accent } else { p.outline }),
        w,
        30.0,
    )
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
                .rect_filled(rect, CornerRadius::same(R_SM), p.accent_soft);
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
    // A card under the cursor is one tonal step up, and one more while pressed.
    let fill = if resp.is_pointer_button_down_on() {
        tint(fill, p.press_layer)
    } else {
        fill
    };
    let stroke = if selected {
        Stroke::new(2.0, p.accent)
    } else if hovered {
        Stroke::new(1.0, p.border_strong)
    } else {
        Stroke::new(1.0, p.outline)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(R_CARD),
        fill,
        stroke,
        StrokeKind::Inside,
    );
    (rect, resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_ranges_are_recognised() {
        assert!(is_arabic('ح'));
        assert!(is_arabic('\u{0660}'));
        assert!(is_arabic('\u{FE8F}'));
        assert!(!is_arabic('A'));
        assert!(!is_arabic(':'));
        assert!(!is_arabic('\\'));
    }

    #[test]
    fn pure_text_is_a_single_run() {
        assert_eq!(bidi_runs("حذف كامل"), vec![(true, "حذف كامل".to_string())]);
        assert_eq!(bidi_runs("AppSweep"), vec![(false, "AppSweep".to_string())]);
        assert!(bidi_runs("").is_empty());
    }

    #[test]
    fn a_state_layer_lands_between_fill_and_layer() {
        let dark = crate::theme::Palette::new(
            crate::settings::ThemeMode::Dark,
            crate::settings::Accent::Blue,
        );
        // A press must lighten the surface, not swallow it.
        let pressed = tint(dark.card, dark.press_layer);
        assert_ne!(pressed, dark.card);
        assert!(pressed.r() > dark.card.r());
        assert_eq!(tint(dark.card, Color32::TRANSPARENT), dark.card);
        assert_eq!(tint(dark.card, Color32::WHITE), Color32::WHITE);
    }

    #[test]
    fn runs_always_rebuild_the_original_string() {
        // Each run is painted separately, so a dropped or reordered character
        // here would silently corrupt every mixed label and path.
        for s in [
            "المسار C:\\Program Files\\App",
            "تم حذف 5 عناصر",
            "C:\\tmp\\ملف.txt",
            "حذف نهائي الآن",
            "#449",
        ] {
            let joined: String = bidi_runs(s).iter().map(|(_, t)| t.as_str()).collect();
            assert_eq!(joined, s);
        }
    }

    #[test]
    fn a_latin_path_inside_arabic_stays_one_whole_run() {
        let runs = bidi_runs("المسار C:\\App\\demo.exe");
        let latin: Vec<&str> = runs
            .iter()
            .filter(|(arabic, _)| !*arabic)
            .map(|(_, text)| text.as_str())
            .collect();
        assert_eq!(latin, vec!["C:\\App\\demo.exe"]);
    }
}
