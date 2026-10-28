//! Card grid view.

use super::widgets::{self, Act, Ctx};
use crate::model::{human_size, AppEntry};
use crate::theme;
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Ui};

const PAD: f32 = 16.0;
const CHIP_ROW: f32 = 24.0;
const ACTION_H: f32 = 30.0;
/// A chip narrower than this is dropped instead of drawn as an ellipsis.
const CHIP_MIN: f32 = 24.0;

pub fn card_height(icon: f32) -> f32 {
    PAD * 2.0 + icon.max(40.0) + 8.0 + CHIP_ROW + 10.0 + ACTION_H
}

/// Width still available for the next chip: it must stop at the card padding.
fn chip_room(card_right: f32, chip_x: f32) -> f32 {
    (card_right - PAD - chip_x).max(0.0)
}

pub fn card(
    ui: &mut Ui,
    ctx: &Ctx,
    app: &AppEntry,
    width: f32,
    selected: bool,
    icon_px: u32,
) -> Option<Act> {
    let p = ctx.palette;
    let icon = ctx_icon(ctx);
    let h = card_height(icon);
    let (rect, resp) = widgets::card_rect_style(ui, ctx, width, h, selected);
    if !ui.is_rect_visible(rect) {
        return None;
    }
    let mut act: Option<Act> = None;
    if resp.clicked() {
        act = Some(Act::Select(app.id));
    }

    // --- icon (left side)
    let isz = icon.max(32.0);
    let icon_rect = Rect::from_min_size(pos2(rect.left() + PAD, rect.top() + PAD), vec2(isz, isz));
    widgets::paint_icon(ui, ctx, app, icon_rect, icon_px);
    // Clicking the icon opens the full details sheet.
    if ui
        .interact(
            icon_rect,
            ui.auto_id_with(("cardicon", app.id)),
            Sense::click(),
        )
        .on_hover_text("Show details")
        .clicked()
    {
        act = Some(Act::Details(app.id));
    }

    // --- star (top right)
    let star_rect = Rect::from_min_size(
        pos2(rect.right() - PAD - 32.0, rect.top() + PAD - 8.0),
        vec2(32.0, 32.0),
    );
    if widgets::star(ui, ("star", app.id), star_rect, app.favorite, p)
        .on_hover_text("Favorite")
        .clicked()
    {
        act = Some(Act::Favorite(app.id));
    }

    // --- name and publisher, to the right of the icon
    let text_left = icon_rect.right() + 10.0;
    let text_right = rect.right() - PAD - 36.0;
    let name_rect = Rect::from_min_max(
        pos2(text_left, rect.top() + PAD + 2.0),
        pos2(text_right, rect.top() + PAD + 22.0),
    );
    if name_rect.width() > 10.0 {
        widgets::text_fit(
            ui,
            name_rect,
            &app.name,
            theme::bold(theme::T_TITLE),
            p.text,
            Align2::LEFT_CENTER,
        );
    }
    let pub_text = if app.publisher.is_empty() {
        app.source.label().to_string()
    } else {
        app.publisher.clone()
    };
    let pub_rect = Rect::from_min_max(
        pos2(text_left, name_rect.bottom() + 2.0),
        pos2(text_right, name_rect.bottom() + 22.0),
    );
    if pub_rect.width() > 10.0 {
        widgets::text_fit(
            ui,
            pub_rect,
            &pub_text,
            FontId::proportional(theme::T_BODY),
            p.text_dim,
            Align2::LEFT_CENTER,
        );
    }

    // --- chips row
    let chip_y = rect.top() + PAD + isz + 8.0;
    let mut chip_x = rect.left() + PAD;
    let chip_font = 11.0;
    let meta_bg = if p.dark {
        egui::Color32::from_rgb(32, 39, 52)
    } else {
        egui::Color32::from_rgb(238, 241, 247)
    };

    // Numeric id, first chip: this is what the delete gate asks for.
    {
        let r = widgets::chip_left(
            ui,
            chip_x,
            chip_y,
            &format!("#{}", app.id),
            p.text_faint,
            if p.dark {
                egui::Color32::from_rgb(26, 32, 43)
            } else {
                egui::Color32::from_rgb(243, 245, 249)
            },
            chip_font,
            60.0,
        );
        chip_x = r.right() + 6.0;
    }

    // Optional chips are dropped when the card is too narrow for them.
    let size_text = human_size(app.size_bytes);
    if size_text != "—" && chip_room(rect.right(), chip_x) >= CHIP_MIN {
        let room = chip_room(rect.right(), chip_x);
        let r = widgets::chip_left(
            ui, chip_x, chip_y, &size_text, p.text_dim, meta_bg, chip_font, room,
        );
        chip_x = r.right() + 6.0;
    }
    if chip_room(rect.right(), chip_x) >= CHIP_MIN {
        let room = chip_room(rect.right(), chip_x);
        let src_bg = if app.is_system {
            p.warn_soft
        } else {
            p.accent_soft
        };
        let src_fg = if app.is_system { p.warn } else { p.accent_text };
        let r = widgets::chip_left(
            ui,
            chip_x,
            chip_y,
            app.source.short(),
            src_fg,
            src_bg,
            chip_font,
            room,
        );
        chip_x = r.right() + 6.0;
    }
    let store_source = app.source == crate::model::Source::Uwp;
    if app.is_system && !store_source && chip_room(rect.right(), chip_x) >= CHIP_MIN {
        let room = chip_room(rect.right(), chip_x);
        let r = widgets::chip_left(
            ui,
            chip_x,
            chip_y,
            "System",
            p.warn,
            p.warn_soft,
            chip_font,
            room,
        );
        chip_x = r.right() + 6.0;
    }
    if app.is_remnant() && chip_room(rect.right(), chip_x) >= CHIP_MIN {
        let room = chip_room(rect.right(), chip_x);
        widgets::chip_left(
            ui,
            chip_x,
            chip_y,
            "Remnant",
            p.danger,
            p.danger_soft,
            chip_font,
            room,
        );
    }

    // --- action row
    let btn_y = rect.bottom() - PAD - ACTION_H;
    let btn_h = ACTION_H;
    let mut x = rect.left() + PAD;

    // Folder button on the far left.
    let folder = Rect::from_min_size(pos2(x, btn_y), vec2(btn_h, btn_h));
    let folder_resp = widgets::pill(
        ui,
        ("reveal", app.id),
        folder,
        "⌸",
        p.text_dim,
        if p.dark {
            egui::Color32::from_rgb(30, 37, 50)
        } else {
            egui::Color32::from_rgb(240, 243, 248)
        },
        p.card_hover,
        15.0,
    );
    if folder_resp.on_hover_text("Open the app folder").clicked() {
        act = Some(Act::Reveal(app.id));
    }
    x += btn_h + 8.0;

    let purge_w = 78.0;
    let purge_rect = Rect::from_min_size(pos2(x, btn_y), vec2(purge_w, btn_h));
    // A protected folder must never look deletable.
    let blocked = app.blocked_reason();
    let (purge_fg, purge_bg, purge_hover) = match blocked {
        Some(_) => (p.text_faint, p.card, p.card),
        None => (
            p.danger,
            p.danger_soft,
            egui::Color32::from_rgba_unmultiplied(240, 100, 112, 60),
        ),
    };
    if widgets::pill(
        ui,
        ("purge", app.id),
        purge_rect,
        "Purge",
        purge_fg,
        purge_bg,
        purge_hover,
        12.0,
    )
    .on_hover_text(blocked.unwrap_or("Permanent deletion of every root and leftover"))
    .clicked()
    {
        act = Some(Act::Purge(app.id));
    }
    x += purge_w + 8.0;

    let un_w = ((rect.right() - PAD) - x).max(52.0);
    let un_rect = Rect::from_min_size(pos2(x, btn_y), vec2(un_w, btn_h));
    let (un_fg, un_bg, un_hover) = match blocked {
        Some(_) => (p.text_faint, p.card, p.card),
        None => (
            p.accent_text,
            p.accent_soft,
            egui::Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 70),
        ),
    };
    let uninstalled = widgets::pill(
        ui,
        ("uninstall", app.id),
        un_rect,
        "Uninstall",
        un_fg,
        un_bg,
        un_hover,
        12.0,
    );
    if uninstalled
        .on_hover_text(blocked.unwrap_or("Run the uninstaller with automatic fallbacks"))
        .clicked()
    {
        act = Some(Act::Uninstall(app.id));
    }

    // --- right click menu with the secondary actions
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
        if ui.button("Open the app folder").clicked() {
            act = Some(Act::Reveal(id));
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

    act
}

fn ctx_icon(ctx: &Ctx) -> f32 {
    ctx.icon
}

/// Lay out the cards in a wrapping grid.
pub fn grid(
    ui: &mut Ui,
    ctx: &Ctx,
    apps: &[&AppEntry],
    selected: Option<u64>,
    icon_px: u32,
    gap: f32,
    min_w: f32,
) -> Option<Act> {
    let avail = ui.available_width();
    let mut cols = ((avail + gap) / (min_w + gap)).floor().max(1.0) as usize;
    let mut w = (avail - gap * (cols.saturating_sub(1)) as f32) / cols as f32;
    // Avoid absurdly wide cards on very wide windows.
    let max_w = 460.0;
    if w > max_w {
        w = max_w;
        cols = ((avail + gap) / (w + gap)).floor().max(1.0) as usize;
    }
    let mut act = None;
    let mut row: Vec<&AppEntry> = Vec::new();
    let mut idx = 0usize;
    for app in apps {
        row.push(app);
        idx += 1;
        if row.len() == cols || idx == apps.len() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for a in &row {
                    if let Some(x) = card(ui, ctx, a, w, selected == Some(a.id), icon_px) {
                        act = Some(x);
                    }
                }
            });
            row.clear();
        }
    }
    act
}

/// Small helper so callers can compute the icon pixel size from the display size.
pub fn icon_px(display: f32) -> u32 {
    ((display * 1.5).round() as u32).clamp(24, 256)
}

/// Unused but handy when tuning: a single-selectable line of chips.
#[allow(dead_code)]
pub fn chip_line(ui: &mut Ui, ctx: &Ctx, labels: &[&str]) {
    ui.horizontal(|ui| {
        for l in labels {
            widgets::chip(ui, l, ctx.palette.text_dim, ctx.palette.accent_soft);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_gets_taller_with_a_bigger_icon() {
        assert_eq!(card_height(40.0), 144.0);
        assert_eq!(card_height(96.0) - card_height(48.0), 48.0);
    }

    #[test]
    fn chips_never_overflow_the_card() {
        assert_eq!(chip_room(300.0, 100.0), 184.0);
        assert_eq!(chip_room(300.0, 290.0), 0.0);
        // A caller that already overran gets no room rather than a negative one.
        assert_eq!(chip_room(300.0, 400.0), 0.0);
    }
}
