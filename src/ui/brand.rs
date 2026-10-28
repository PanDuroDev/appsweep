//! Brand marks for apps whose icon cannot be extracted from disk.
//!
//! Rasterised once from Simple Icons (CC0) at 64 px with the brand colour baked
//! in, then embedded: no network at runtime, still a single exe. Only apps that
//! resolve to no file at all (remnants, uninstalled entries, bare system entries)
//! reach this path, because a real icon always wins.

use egui::{ColorImage, Rect, TextureHandle, TextureOptions, Ui};
use std::cell::RefCell;
use std::collections::HashMap;

/// Keys are matched against a normalised app name, whole words only.
static BRANDS: &[(&[&str], &[u8])] = &[
    (
        &["7 zip", "7zip"],
        include_bytes!("../../assets/brand/7zip.png"),
    ),
    (
        &["anydesk"],
        include_bytes!("../../assets/brand/anydesk.png"),
    ),
    (
        &["audacity"],
        include_bytes!("../../assets/brand/audacity.png"),
    ),
    (
        &["blender"],
        include_bytes!("../../assets/brand/blender.png"),
    ),
    (&["brave"], include_bytes!("../../assets/brand/brave.png")),
    (
        &["davinci resolve", "davinci"],
        include_bytes!("../../assets/brand/davinciresolve.png"),
    ),
    (
        &["discord"],
        include_bytes!("../../assets/brand/discord.png"),
    ),
    (&["docker"], include_bytes!("../../assets/brand/docker.png")),
    (
        &["dropbox"],
        include_bytes!("../../assets/brand/dropbox.png"),
    ),
    (
        &["epic games"],
        include_bytes!("../../assets/brand/epicgames.png"),
    ),
    (
        &["evernote"],
        include_bytes!("../../assets/brand/evernote.png"),
    ),
    (&["figma"], include_bytes!("../../assets/brand/figma.png")),
    (
        &["firefox"],
        include_bytes!("../../assets/brand/firefox.png"),
    ),
    (&["gimp"], include_bytes!("../../assets/brand/gimp.png")),
    // GitHub first: "github desktop" also contains the word "git".
    (&["github"], include_bytes!("../../assets/brand/github.png")),
    (&["git"], include_bytes!("../../assets/brand/git.png")),
    (
        &["inkscape"],
        include_bytes!("../../assets/brand/inkscape.png"),
    ),
    (&["krita"], include_bytes!("../../assets/brand/krita.png")),
    (
        &["libreoffice"],
        include_bytes!("../../assets/brand/libreoffice.png"),
    ),
    (
        &["mariadb", "mysql"],
        include_bytes!("../../assets/brand/mysql.png"),
    ),
    (
        &["node.js", "nodejs", "node js"],
        include_bytes!("../../assets/brand/nodedotjs.png"),
    ),
    (
        &["notepad++"],
        include_bytes!("../../assets/brand/notepadplusplus.png"),
    ),
    (&["notion"], include_bytes!("../../assets/brand/notion.png")),
    (
        &["obsidian"],
        include_bytes!("../../assets/brand/obsidian.png"),
    ),
    (
        &["obs studio", "obs"],
        include_bytes!("../../assets/brand/obsstudio.png"),
    ),
    (
        &["java", "openjdk", "jdk", "jre"],
        include_bytes!("../../assets/brand/openjdk.png"),
    ),
    (&["opera"], include_bytes!("../../assets/brand/opera.png")),
    (
        &["postgresql", "postgres"],
        include_bytes!("../../assets/brand/postgresql.png"),
    ),
    (&["python"], include_bytes!("../../assets/brand/python.png")),
    (
        &["spotify"],
        include_bytes!("../../assets/brand/spotify.png"),
    ),
    (&["steam"], include_bytes!("../../assets/brand/steam.png")),
    (
        &["teamviewer"],
        include_bytes!("../../assets/brand/teamviewer.png"),
    ),
    (
        &["telegram"],
        include_bytes!("../../assets/brand/telegram.png"),
    ),
    (
        &["thunderbird"],
        include_bytes!("../../assets/brand/thunderbird.png"),
    ),
    (&["unity"], include_bytes!("../../assets/brand/unity.png")),
    (
        &["unreal engine", "unreal"],
        include_bytes!("../../assets/brand/unrealengine.png"),
    ),
    (
        &["vivaldi"],
        include_bytes!("../../assets/brand/vivaldi.png"),
    ),
    (
        &["vlc"],
        include_bytes!("../../assets/brand/vlcmediaplayer.png"),
    ),
    (
        &["whatsapp"],
        include_bytes!("../../assets/brand/whatsapp.png"),
    ),
    (&["zoom"], include_bytes!("../../assets/brand/zoom.png")),
];

/// Lowercase, keep letters/digits and the characters that carry meaning in an
/// app name, and pad with spaces so a key can be matched as whole words only.
fn normalise(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    out.push(' ');
    let mut space = true;
    for c in name.chars() {
        if c.is_alphanumeric() || c == '.' || c == '+' {
            out.extend(c.to_lowercase());
            space = false;
        } else if !space {
            out.push(' ');
            space = true;
        }
    }
    out.push(' ');
    out
}

/// Index into `BRANDS` for an app name, or `None`.
pub fn lookup(name: &str) -> Option<usize> {
    let hay = normalise(name);
    BRANDS
        .iter()
        .position(|(keys, _)| keys.iter().any(|k| hay.contains(&format!(" {k} "))))
}

thread_local! {
    static CACHE: RefCell<HashMap<usize, TextureHandle>> = RefCell::new(HashMap::new());
}

fn texture(ctx: &egui::Context, index: usize) -> TextureHandle {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(done) = cache.get(&index) {
            return done.clone();
        }
        let png = image::load_from_memory(BRANDS[index].1)
            .expect("embedded brand mark")
            .to_rgba8();
        let image = ColorImage::from_rgba_unmultiplied(
            [png.width() as usize, png.height() as usize],
            png.as_raw(),
        );
        let handle = ctx.load_texture(format!("brand-{index}"), image, TextureOptions::LINEAR);
        cache.insert(index, handle.clone());
        handle
    })
}

/// Paint the brand mark for `name` into `rect`. Returns false when we ship none.
pub fn paint(ui: &Ui, rect: Rect, name: &str) -> bool {
    let Some(index) = lookup(name) else {
        return false;
    };
    let handle = texture(ui.ctx(), index);
    ui.painter().image(
        handle.id(),
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_normalise_to_matchable_words() {
        assert_eq!(normalise("Notepad++"), " notepad++ ");
        assert_eq!(normalise("Node.js"), " node.js ");
        assert_eq!(normalise("7-Zip 24.08"), " 7 zip 24.08 ");
    }

    #[test]
    fn short_keys_only_match_whole_words() {
        // "Digital" contains "git" as a substring; it must not get the Git mark.
        assert_eq!(lookup("Digital Clock"), None);
        assert_eq!(lookup("GitHub Desktop"), lookup("github"));
        assert_eq!(lookup("Git for Windows"), lookup("git"));
    }

    #[test]
    fn the_common_brands_resolve() {
        for name in [
            "Firefox",
            "Discord",
            "Python 3.12",
            "Steam",
            "VLC media player",
            "Spotify",
            "7-Zip",
            "Notepad++",
            "Notion",
            "Docker Desktop",
        ] {
            assert!(lookup(name).is_some(), "{name} should resolve");
        }
    }

    #[test]
    fn every_table_entry_has_keys_and_bytes() {
        for (keys, png) in BRANDS {
            assert!(!keys.is_empty());
            assert!(png.starts_with(&[0x89, b'P', b'N', b'G']), "not a png");
        }
    }
}
