use crate::settings::{Accent, Density, ThemeMode};
use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

// Material 3 shape scale, in points.
pub const R_XS: u8 = 8; // chips, badges
pub const R_SM: u8 = 12; // inputs, small frames
pub const R_CARD: u8 = 16; // cards, option rows
pub const R_LG: u8 = 20; // panels
pub const R_XL: u8 = 28; // dialogs
pub const R_PILL: u8 = 255; // fully rounded: buttons, filter pills (egui clamps to half the height)

// Material 3 type scale, in points.
/// Height of every interactive control in a bar: pills, the search field and
/// the sort combo. One number so they line up instead of each picking its own.
pub const CONTROL_H: f32 = 30.0;

pub const T_LABEL: f32 = 11.0;
pub const T_BODY: f32 = 12.0;
pub const T_UI: f32 = 14.0;
pub const T_TITLE: f32 = 16.0;
pub const T_HEADLINE: f32 = 20.0;

#[derive(Clone)]
pub struct Palette {
    pub bg: Color32,
    pub bg_alt: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    /// Boundary of an interactive control. `border` is decoration and may stay
    /// faint; this one has to clear 3:1 against the surfaces it sits on.
    pub outline: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub accent_text: Color32,
    /// Label on the raw accent fill, picked by measured contrast.
    pub on_accent: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
    /// Label on a filled danger surface, picked by measured contrast.
    pub on_danger: Color32,
    pub warn: Color32,
    pub warn_soft: Color32,
    pub ok: Color32,
    pub ok_soft: Color32,
    /// Translucent overlay for hover and press, so a control keeps its own
    /// fill colour while still showing the pointer state.
    pub hover_layer: Color32,
    pub press_layer: Color32,
    pub dark: bool,
}

impl Palette {
    pub fn new(mode: ThemeMode, accent: Accent) -> Self {
        let (ar, ag, ab) = accent.rgb();
        let accent = Color32::from_rgb(ar, ag, ab);
        match mode {
            ThemeMode::Dark => Self {
                bg: Color32::from_rgb(17, 17, 17),
                bg_alt: Color32::from_rgb(22, 22, 22),
                panel: Color32::from_rgb(26, 26, 26),
                card: Color32::from_rgb(30, 30, 30),
                card_hover: Color32::from_rgb(38, 38, 38),
                border: Color32::from_rgb(45, 45, 45),
                border_strong: Color32::from_rgb(68, 68, 68),
                outline: Color32::from_rgb(122, 122, 122),
                text: Color32::from_rgb(234, 234, 234),
                text_dim: Color32::from_rgb(176, 176, 176),
                text_faint: Color32::from_rgb(150, 150, 150),
                accent,
                accent_soft: Color32::from_rgba_unmultiplied(ar, ag, ab, 38),
                accent_text: Color32::from_rgb(
                    (ar as u16 + 60).min(255) as u8,
                    (ag as u16 + 50).min(255) as u8,
                    (ab as u16 + 40).min(255) as u8,
                ),
                on_accent: Color32::from_rgb(11, 14, 20),
                danger: Color32::from_rgb(255, 120, 132),
                danger_soft: Color32::from_rgba_unmultiplied(240, 100, 112, 34),
                on_danger: Color32::from_rgb(11, 14, 20),
                warn: Color32::from_rgb(238, 178, 78),
                warn_soft: Color32::from_rgba_unmultiplied(238, 178, 78, 32),
                ok: Color32::from_rgb(78, 198, 138),
                ok_soft: Color32::from_rgba_unmultiplied(78, 198, 138, 32),
                hover_layer: Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                press_layer: Color32::from_rgba_unmultiplied(255, 255, 255, 31),
                dark: true,
            },
            ThemeMode::Light => Self {
                bg: Color32::from_rgb(222, 222, 222),
                bg_alt: Color32::from_rgb(226, 226, 226),
                panel: Color32::from_rgb(234, 234, 234),
                card: Color32::from_rgb(234, 234, 234),
                card_hover: Color32::from_rgb(242, 242, 242),
                border: Color32::from_rgb(212, 212, 212),
                border_strong: Color32::from_rgb(186, 186, 186),
                outline: Color32::from_rgb(116, 116, 116),
                text: Color32::from_rgb(17, 17, 17),
                text_dim: Color32::from_rgb(68, 68, 68),
                text_faint: Color32::from_rgb(88, 88, 88),
                accent,
                accent_soft: Color32::from_rgba_unmultiplied(ar, ag, ab, 26),
                // Light surfaces need a much darker ink than the accent itself:
                // these are the darkest-but-same-hue points where the soft fill
                // still clears 4.5:1 once it is composited over every surface.
                accent_text: ink_on_light(accent),
                on_accent: readable_on(accent),
                danger: Color32::from_rgb(162, 48, 58),
                danger_soft: Color32::from_rgba_unmultiplied(203, 60, 74, 22),
                on_danger: readable_on(Color32::from_rgb(162, 48, 58)),
                warn: Color32::from_rgb(122, 86, 14),
                warn_soft: Color32::from_rgba_unmultiplied(200, 150, 40, 26),
                ok: Color32::from_rgb(22, 103, 68),
                ok_soft: Color32::from_rgba_unmultiplied(30, 140, 92, 22),
                hover_layer: Color32::from_rgba_unmultiplied(0, 0, 0, 20),
                press_layer: Color32::from_rgba_unmultiplied(0, 0, 0, 31),
                dark: false,
            },
        }
    }
}

/// Composite a translucent colour onto an opaque one.
#[cfg(test)]
pub fn over(fg: Color32, bg: Color32) -> Color32 {
    // Color32 stores premultiplied channels, so the foreground is added as-is.
    let a = fg.a() as f32 / 255.0;
    let mix = |f: u8, b: u8| (f as f32 + b as f32 * (1.0 - a)).round().min(255.0) as u8;
    Color32::from_rgb(
        mix(fg.r(), bg.r()),
        mix(fg.g(), bg.g()),
        mix(fg.b(), bg.b()),
    )
}

fn luminance(c: Color32) -> f32 {
    let ch = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
}

/// WCAG 2.1 contrast ratio between two opaque colours, 1.0 .. 21.0.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = (la.max(lb), la.min(lb));
    (hi + 0.05) / (lo + 0.05)
}

/// Black or near-black, whichever is more readable on `bg`.
pub fn readable_on(bg: Color32) -> Color32 {
    let dark = Color32::from_rgb(11, 14, 20);
    let light = Color32::from_rgb(255, 255, 255);
    if contrast(dark, bg) >= contrast(light, bg) {
        dark
    } else {
        light
    }
}

/// Ink for text sitting on a light theme's `accent_soft` fill.
fn ink_on_light(accent: Color32) -> Color32 {
    match (accent.r(), accent.g(), accent.b()) {
        (56, 132, 255) => Color32::from_rgb(36, 84, 164),
        (32, 178, 170) => Color32::from_rgb(18, 99, 94),
        (139, 108, 246) => Color32::from_rgb(93, 72, 165),
        (233, 168, 60) => Color32::from_rgb(117, 84, 30),
        (232, 93, 124) => Color32::from_rgb(145, 58, 77),
        (58, 183, 120) => Color32::from_rgb(33, 101, 68),
        other => Color32::from_rgb(other.0 * 3 / 4, other.1 * 3 / 4, other.2 * 3 / 4),
    }
}

pub fn apply(ctx: &egui::Context, mode: ThemeMode, accent: Accent, density: Density) {
    let p = Palette::new(mode, accent);
    let mut style = if p.dark {
        egui::Style::default()
    } else {
        egui::Style {
            visuals: egui::Visuals::light(),
            ..Default::default()
        }
    };

    let mut visuals = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = p.bg;
    visuals.window_fill = p.panel;
    // Also the scroll-bar track, which is painted whenever `floating` is off.
    // The old values were a leftover blue-black and pure white, both of which
    // showed up as a coloured strip against the neutral page.
    visuals.extreme_bg_color = if p.dark {
        Color32::from_rgb(12, 12, 12)
    } else {
        Color32::from_rgb(226, 226, 226)
    };
    visuals.faint_bg_color = p.bg_alt;
    visuals.window_stroke = Stroke::new(1.0, p.border);
    visuals.window_corner_radius = CornerRadius::same(R_LG);
    visuals.menu_corner_radius = CornerRadius::same(R_SM);
    visuals.selection.bg_fill = p.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, p.accent);
    visuals.hyperlink_color = p.accent;
    visuals.warn_fg_color = p.warn;
    visuals.error_fg_color = p.danger;

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.bg_alt;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text_dim);
    w.noninteractive.corner_radius = CornerRadius::same(R_SM);

    // `bg_fill` is also the scroll-bar handle (the track is `extreme_bg_color`),
    // so it has to differ from both the page and the track. Buttons use
    // `weak_bg_fill`, which stays a surface tone.
    w.inactive.bg_fill = if p.dark {
        Color32::from_rgb(34, 34, 34)
    } else {
        Color32::from_rgb(200, 200, 200)
    };
    w.inactive.weak_bg_fill = if p.dark {
        Color32::from_rgb(34, 34, 34)
    } else {
        Color32::from_rgb(226, 226, 226)
    };
    w.inactive.bg_stroke = Stroke::new(1.0, p.outline);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = CornerRadius::same(R_SM);

    w.hovered.bg_fill = p.card_hover;
    w.hovered.weak_bg_fill = p.card_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.outline);
    w.hovered.fg_stroke = Stroke::new(1.5, p.text);
    w.hovered.corner_radius = CornerRadius::same(R_SM);

    w.active.bg_fill = p.accent_soft;
    w.active.weak_bg_fill = p.accent_soft;
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(1.5, p.text);
    w.active.corner_radius = CornerRadius::same(R_SM);

    w.open.bg_fill = p.card_hover;
    w.open.weak_bg_fill = p.card_hover;
    w.open.bg_stroke = Stroke::new(1.0, p.outline);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = CornerRadius::same(R_SM);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    // The settings sliders sit in a narrow column next to their labels.
    style.spacing.slider_width = 160.0;
    style.spacing.menu_margin = egui::Margin::same(8);
    style.spacing.indent = 16.0;
    style.spacing.scroll.bar_width = 9.0;
    style.spacing.scroll.floating = false;
    style.spacing.interact_size.y = match density {
        Density::Compact => 24.0,
        Density::Normal => 28.0,
        Density::Relaxed => 34.0,
    };

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(22.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.5, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.5, FontFamily::Monospace),
    );

    ctx.all_styles_mut(|s| *s = style.clone());
    ctx.set_theme(if p.dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
}

/// Bold text helper (uses the "bold" family when the font was loaded).
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn load_fonts(ctx: &egui::Context) {
    use std::sync::Arc;
    let mut fonts = egui::FontDefinitions::default();

    let candidates: [(&str, &[&str]); 2] = [
        (
            "ar",
            &[
                "C:\\Windows\\Fonts\\segoeui.ttf",
                "C:\\Windows\\Fonts\\tahoma.ttf",
            ],
        ),
        (
            "arb",
            &[
                "C:\\Windows\\Fonts\\segoeuib.ttf",
                "C:\\Windows\\Fonts\\tahomabd.ttf",
            ],
        ),
    ];

    for (name, paths) in candidates {
        for p in paths {
            if let Ok(bytes) = std::fs::read(p) {
                fonts.font_data.insert(
                    name.to_string(),
                    Arc::new(egui::FontData::from_owned(bytes)),
                );
                break;
            }
        }
        if fonts.font_data.contains_key(name) {
            let fam = if name == "ar" {
                FontFamily::Proportional
            } else {
                FontFamily::Name("bold".into())
            };
            fonts
                .families
                .entry(fam)
                .or_default()
                .insert(0, name.to_string());
        }
    }

    // Symbol font: Segoe UI has no glyphs for stars, check marks, folder marks...
    // Segoe UI Symbol covers all of them, so append it as a last-resort fallback.
    for p in [
        "C:\\Windows\\Fonts\\seguisym.ttf",
        "C:\\Windows\\Fonts\\seguisb.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(p) {
            fonts.font_data.insert(
                "sym".to_string(),
                Arc::new(egui::FontData::from_owned(bytes)),
            );
            break;
        }
    }
    if fonts.font_data.contains_key("sym") {
        for fam in [
            FontFamily::Proportional,
            FontFamily::Monospace,
            FontFamily::Name("bold".into()),
        ] {
            fonts
                .families
                .entry(fam)
                .or_default()
                .push("sym".to_string());
        }
    }

    // Keep a monospace fallback that also has Arabic glyphs.
    if fonts.font_data.contains_key("ar") {
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .push("ar".to_string());
    }

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_accent_is_the_colour_that_was_asked_for() {
        for a in Accent::ALL {
            let (r, g, b) = a.rgb();
            let p = Palette::new(ThemeMode::Dark, a);
            assert_eq!(p.accent, Color32::from_rgb(r, g, b));
        }
    }

    #[test]
    fn dark_and_light_are_actually_inverted() {
        let d = Palette::new(ThemeMode::Dark, Accent::Blue);
        let l = Palette::new(ThemeMode::Light, Accent::Blue);
        assert!(d.dark);
        assert!(!l.dark);
        assert_ne!(d.bg, l.bg);
        assert_ne!(d.card, l.card);
        assert_ne!(d.text, l.text);
        // A swapped palette would make the whole UI unreadable.
        assert!(d.text.r() > 128);
        assert!(l.text.r() < 128);
    }

    #[test]
    fn status_colours_do_not_collide() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let p = Palette::new(mode, Accent::Blue);
            assert_ne!(p.danger, p.warn);
            assert_ne!(p.warn, p.ok);
            assert_ne!(p.danger, p.ok);
            assert_ne!(p.text, p.text_dim);
            assert_ne!(p.text_dim, p.text_faint);
            assert_ne!(p.card, p.card_hover);
        }
    }

    #[test]
    fn accent_text_is_brighter_than_the_fill_in_dark_mode() {
        for a in Accent::ALL {
            let p = Palette::new(ThemeMode::Dark, a);
            assert!(p.accent_text.r() >= p.accent.r());
            assert!(p.accent_text.g() >= p.accent.g());
        }
    }

    #[test]
    fn shape_and_type_scales_are_ordered() {
        // A stray out-of-scale value is invisible in code review but shows up
        // as one control with a different corner or size than its neighbours.
        assert!(R_XS < R_SM && R_SM < R_CARD && R_CARD < R_LG && R_LG < R_XL);
        assert!(R_PILL > R_XL);
        assert!(T_LABEL < T_BODY && T_BODY < T_UI && T_UI < T_TITLE && T_TITLE < T_HEADLINE);
    }

    #[test]
    fn state_layers_stay_translucent() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let p = Palette::new(mode, Accent::Blue);
            // Material 3 asks for 8 % hover and 12 % press; alpha is 0-255.
            assert!((19..=21).contains(&p.hover_layer.a()), "{mode:?} hover");
            assert!((30..=32).contains(&p.press_layer.a()), "{mode:?} press");
            assert!(
                p.press_layer.a() < 60,
                "a state layer must never hide the fill"
            );
        }
    }

    #[test]
    fn the_page_sits_behind_the_cards() {
        let light = Palette::new(ThemeMode::Light, Accent::Blue);
        // White cards on a white page have no edge; the page must be darker.
        assert!(luminance(light.bg) < luminance(light.card));
        let dark = Palette::new(ThemeMode::Dark, Accent::Blue);
        assert!(luminance(dark.bg) < luminance(dark.card));
    }
    #[test]
    fn every_text_role_clears_wcag_aa() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            for accent in Accent::ALL {
                let p = Palette::new(mode, accent);
                for bg in [p.bg, p.bg_alt, p.panel, p.card, p.card_hover] {
                    for (role, fg) in [
                        ("text", p.text),
                        ("text_dim", p.text_dim),
                        ("text_faint", p.text_faint),
                    ] {
                        let r = contrast(fg, bg);
                        assert!(r >= 4.5, "{mode:?} {accent:?}: {role} on {bg:?} is {r:.2}");
                    }
                }
                // The accent ink is used on its own translucent fill and as a
                // plain text colour, so both have to clear the bar.
                for bg in [p.panel, p.card, p.card_hover, p.bg, p.bg_alt] {
                    let soft = over(p.accent_soft, bg);
                    let r = contrast(p.accent_text, soft);
                    assert!(
                        r >= 4.5,
                        "{mode:?} {accent:?}: accent_text on soft over {bg:?} is {r:.2}"
                    );
                    let r = contrast(p.accent_text, bg);
                    assert!(
                        r >= 4.5,
                        "{mode:?} {accent:?}: accent_text on {bg:?} is {r:.2}"
                    );
                }
                // Labels on filled surfaces, and semantic ink on its own fill.
                let r = contrast(p.on_accent, p.accent);
                assert!(r >= 4.5, "{mode:?} {accent:?}: on_accent is {r:.2}");
                let r = contrast(p.on_danger, p.danger);
                assert!(r >= 4.5, "{mode:?} {accent:?}: on_danger is {r:.2}");
                for bg in [p.panel, p.card, p.card_hover] {
                    for (name, ink, soft) in [
                        ("danger", p.danger, p.danger_soft),
                        ("warn", p.warn, p.warn_soft),
                        ("ok", p.ok, p.ok_soft),
                    ] {
                        let r = contrast(ink, over(soft, bg));
                        assert!(
                            r >= 4.5,
                            "{mode:?} {accent:?}: {name} on its fill over {bg:?} is {r:.2}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn control_outlines_clear_the_three_to_one_bar() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            for accent in Accent::ALL {
                let p = Palette::new(mode, accent);
                for bg in [p.bg, p.bg_alt, p.panel, p.card, p.card_hover] {
                    let r = contrast(p.outline, bg);
                    assert!(r >= 3.0, "{mode:?} {accent:?}: outline on {bg:?} is {r:.2}");
                }
            }
        }
    }
    #[test]
    fn the_base_palette_is_neutral_and_inverted() {
        // The requested scheme: dark page #111111 with #EAEAEA text, light
        // surface #EAEAEA with #111111 text.
        let d = Palette::new(ThemeMode::Dark, Accent::Blue);
        let l = Palette::new(ThemeMode::Light, Accent::Blue);
        assert_eq!(d.bg, Color32::from_rgb(0x11, 0x11, 0x11));
        assert_eq!(d.text, Color32::from_rgb(0xEA, 0xEA, 0xEA));
        assert_eq!(l.card, Color32::from_rgb(0xEA, 0xEA, 0xEA));
        assert_eq!(l.text, Color32::from_rgb(0x11, 0x11, 0x11));
        // Every neutral in the ramp must stay grey, not drift blue like before.
        for p in [d, l] {
            for c in [
                p.bg,
                p.bg_alt,
                p.panel,
                p.card,
                p.card_hover,
                p.text,
                p.text_dim,
                p.text_faint,
            ] {
                assert!(c.r() == c.g() && c.g() == c.b(), "not neutral: {:?}", c);
            }
        }
    }
}
