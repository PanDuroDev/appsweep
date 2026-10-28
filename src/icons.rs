use crate::model::AppEntry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub struct IconJob {
    pub key: String,
    pub path: String,
    pub size: u32,
}

pub struct IconDone {
    pub key: String,
    pub image: Option<egui::ColorImage>,
}

/// Background icon extractor: keeps the UI thread free of Shell API calls.
pub struct IconService {
    tx: Sender<IconJob>,
    rx: Receiver<IconDone>,
    pending: std::collections::HashSet<String>,
}

impl IconService {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, job_rx) = channel::<IconJob>();
        let (done_tx, rx) = channel::<IconDone>();

        thread::Builder::new()
            .name("icons".into())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let raw = crate::native::extract_icon(&job.path, job.size);
                    let image = raw.map(|r| {
                        egui::ColorImage::from_rgba_unmultiplied([r.w, r.h], &r.rgba)
                    });
                    if done_tx
                        .send(IconDone {
                            key: job.key,
                            image,
                        })
                        .is_err()
                    {
                        break;
                    }
                    ctx.request_repaint();
                }
            })
            .ok();

        Self {
            tx,
            rx,
            pending: Default::default(),
        }
    }

    pub fn request_raw(&mut self, id: u64, path: String, size: u32) {
        let key = format!("{}|{}", id, size);
        if self.pending.contains(&key) {
            return;
        }
        self.pending.insert(key.clone());
        let _ = self.tx.send(IconJob { key, path, size });
    }

    pub fn drain(&mut self) -> Vec<IconDone> {
        let mut out = Vec::new();
        while let Ok(d) = self.rx.try_recv() {
            self.pending.remove(&d.key);
            out.push(d);
        }
        out
    }
}

/// Path -> texture cache with per-size keys.
#[derive(Default)]
pub struct IconCache {
    map: HashMap<String, Option<egui::TextureHandle>>,
}


impl IconCache {
    pub fn get(&self, app: &AppEntry, size: u32) -> Option<&egui::TextureHandle> {
        self.map.get(&format!("{}|{}", app.id, size))?.as_ref()
    }

    pub fn contains(&self, app: &AppEntry, size: u32) -> bool {
        self.map.contains_key(&format!("{}|{}", app.id, size))
    }

    pub fn insert(
        &mut self,
        app_id: u64,
        size: u32,
        image: Option<egui::ColorImage>,
        ctx: &egui::Context,
    ) {
        let key = format!("{}|{}", app_id, size);
        let tex = image.map(|img| {
            ctx.load_texture(
                format!("icon-{}", key),
                img,
                egui::TextureOptions::LINEAR,
            )
        });
        self.map.insert(key, tex);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"))
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| home_dir().join("AppData\\Roaming"))
        .join("AppSweep")
}
