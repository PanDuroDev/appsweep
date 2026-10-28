//! Small UI glyphs for buttons.
//!
//! Rasterised once from MingCute (free, Apache-2.0) into 64 px alpha masks and
//! embedded in the binary, so the app keeps working as a single file with no
//! network access at runtime. Each colour gets its own baked texture because a
//! texture carries its tint; there are only a handful of colours per theme.

use egui::{Color32, ColorImage, Rect, TextureHandle, TextureOptions, Ui};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Glyph {
    Search,
    Scan,
    Settings,
    Backups,
    Folder,
    Purge,
    Uninstall,
    Close,
    Cards,
    Table,
}

fn bytes(glyph: Glyph) -> &'static [u8] {
    match glyph {
        Glyph::Search => include_bytes!("../../assets/ui/search.png"),
        Glyph::Scan => include_bytes!("../../assets/ui/scan.png"),
        Glyph::Settings => include_bytes!("../../assets/ui/settings.png"),
        Glyph::Backups => include_bytes!("../../assets/ui/backups.png"),
        Glyph::Folder => include_bytes!("../../assets/ui/folder.png"),
        Glyph::Purge => include_bytes!("../../assets/ui/purge.png"),
        Glyph::Uninstall => include_bytes!("../../assets/ui/uninstall.png"),
        Glyph::Close => include_bytes!("../../assets/ui/close.png"),
        Glyph::Cards => include_bytes!("../../assets/ui/cards.png"),
        Glyph::Table => include_bytes!("../../assets/ui/table.png"),
    }
}

thread_local! {
    static CACHE: RefCell<HashMap<(Glyph, [u8; 4]), TextureHandle>> = RefCell::new(HashMap::new());
}

fn texture(ctx: &egui::Context, glyph: Glyph, color: Color32) -> TextureHandle {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(done) = cache.get(&(glyph, color.to_array())) {
            return done.clone();
        }
        let mask = image::load_from_memory(bytes(glyph))
            .expect("embedded glyph")
            .to_rgba8();
        let mut rgba = Vec::with_capacity(mask.len());
        for px in mask.pixels() {
            rgba.extend_from_slice(&[color.r(), color.g(), color.b(), px[3]]);
        }
        let image = ColorImage::from_rgba_unmultiplied(
            [mask.width() as usize, mask.height() as usize],
            &rgba,
        );
        let handle = ctx.load_texture(
            format!("glyph-{glyph:?}-{color:?}"),
            image,
            TextureOptions::LINEAR,
        );
        cache.insert((glyph, color.to_array()), handle.clone());
        handle
    })
}

/// Paint a glyph into `rect` (a square), tinted `color`.
pub fn paint(ui: &Ui, rect: Rect, glyph: Glyph, color: Color32) {
    let handle = texture(ui.ctx(), glyph, color);
    ui.painter().image(
        handle.id(),
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Paint a glyph centred on `center` at `size` pixels.
pub fn paint_at(ui: &Ui, center: egui::Pos2, size: f32, glyph: Glyph, color: Color32) {
    paint(
        ui,
        Rect::from_center_size(center, egui::vec2(size, size)),
        glyph,
        color,
    );
}
