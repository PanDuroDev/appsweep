//! Modal dialogs: confirmation flow, live job progress, details and settings.

use super::widgets::{self, bidi_line, text_fit};
use crate::actions::{Flow, Job, StepStatus};
use crate::model::{human_size, AppEntry, Source};
use crate::settings::{Accent, Density, Settings, SortKey, ThemeMode, ViewMode};
use crate::theme::{self, Palette};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, Modal,
    ProgressBar, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Desktop,
    Store,
    System,
    Favorites,
    Remnants,
}

impl Filter {
    pub fn label_ar(&self) -> &'static str {
        match self {
            Filter::All => "الكل",
            Filter::Desktop => "سطح المكتب",
            Filter::Store => "المتجر",
            Filter::System => "تطبيقات النظام",
            Filter::Favorites => "المفضلة",
            Filter::Remnants => "بقايا",
        }
    }
}

/// What a dialog asks the app to do next.
pub enum Asked {
    None,
    Close,
    Confirm {
        app_id: u64,
        flow: Flow,
        auto_cascade: bool,
        keep_data: bool,
    },
    OpenFolder(u64),
    Run(u64),
    CopyPath(u64),
    SetNotes {
        app_id: u64,
        notes: String,
    },
    Rescan,
}

fn modal_frame(p: &Palette) -> Frame {
    Frame::NONE
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border_strong))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(20))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 28,
            spread: 0,
            color: Color32::from_black_alpha(if p.dark { 140 } else { 60 }),
        })
}

fn title(ui: &mut Ui, p: &Palette, text: &str) {
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(text).font(theme::bold(18.0)).color(p.text));
        });
    });
}

/// Arabic label on the right, value (usually Latin or numeric) on its left.
fn field(ui: &mut Ui, p: &Palette, label: &str, value: &str, mono: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(label)
                    .font(FontId::proportional(12.5))
                    .color(p.text_faint),
            );
            let font = if mono {
                FontId::monospace(11.5)
            } else {
                FontId::proportional(12.5)
            };
            if value.is_empty() {
                ui.label(RichText::new("—").font(font).color(p.text_faint));
            } else {
                let resp = ui.add(
                    egui::Label::new(RichText::new(value).font(font).color(p.text))
                        .truncate()
                        .sense(Sense::click()),
                );
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    ui.ctx().copy_text(value.to_string());
                }
                resp.on_hover_text("اضغط للنسخ");
            }
        });
    });
}

fn section(ui: &mut Ui, p: &Palette, text: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(text)
                    .font(theme::bold(12.5))
                    .color(p.accent_text),
            );
        });
    });
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0, p.border);
    ui.add_space(2.0);
}

// ---------------------------------------------------------------- confirmation

pub struct ConfirmState {
    pub app: AppEntry,
    pub flow: Flow,
    pub stage: u8,
    pub ack: bool,
    pub auto_cascade: bool,
    pub keep_data: bool,
    /// Text the user must type to unlock the final permanent-delete button.
    pub typed: String,
}

/// True when the text the user typed matches the app's numeric id.
///
/// The id is used instead of the name because app names can contain characters
/// that are hard to reproduce (symbols, private-use glyphs, unusual spacing).
/// An id is short, ASCII and always typeable.
fn typed_matches(st: &ConfirmState) -> bool {
    let got: String = st.typed.chars().filter(|c| c.is_ascii_digit()).collect();
    !got.is_empty() && got.parse::<u64>().ok() == Some(st.app.id)
}

pub fn confirm(ui: &mut Ui, p: &Palette, st: &mut ConfirmState) -> Asked {
    let mut asked = Asked::None;
    let width = 560.0;
    ui.set_width(width);

    let heading = match st.stage {
        0 => "اختيار طريقة الحذف",
        _ => "تأكيد الحذف",
    };
    title(ui, p, heading);
    ui.add_space(6.0);

    // App summary card
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 62.0), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        p.card,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let name_rect = Rect::from_min_max(
        pos2(rect.left() + 14.0, rect.top() + 8.0),
        pos2(rect.right() - 14.0, rect.top() + 30.0),
    );
    text_fit(
        ui,
        name_rect,
        &st.app.name,
        theme::bold(15.0),
        p.text,
        Align2::RIGHT_CENTER,
    );
    let sub = st
        .app
        .install_dir
        .as_ref()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_else(|| "—".to_string());
    let sub_rect = Rect::from_min_max(
        pos2(rect.left() + 14.0, rect.top() + 32.0),
        pos2(rect.right() - 14.0, rect.top() + 54.0),
    );
    text_fit(
        ui,
        sub_rect,
        &sub,
        FontId::monospace(11.0),
        p.text_dim,
        Align2::RIGHT_CENTER,
    );
    ui.add_space(8.0);

    if st.stage == 0 {
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.vertical(|ui| {
                    ui.set_width(width - 40.0);
                    let opts = [
                        (
                            Flow::Uninstall,
                            "إزالة عبر الأداة الرسمية",
                            "يشغّل أداة إزالة التثبيت، ثم يجرّب الإزالة الصامتة، ثم MSI، ثم مدير الحزم.",
                        ),
                        (
                            Flow::FilesOnly,
                            "حذف الملفات فقط",
                            "ينقل مجلد البرنامج إلى سلة المحذوفات ثم يحذفه. لا يمس السجل.",
                        ),
                        (
                            Flow::Purge,
                            "حذف نهائي بكل الجذور",
                            "يحذف الملفات وبيانات التطبيق والسجل والاختصارات والخدمات. لا يمكن التراجع.",
                        ),
                    ];
                    for (flow, label, desc) in opts {
                        let selected = st.flow == flow;
                        let (r, resp) = ui.allocate_exact_size(
                            vec2(ui.available_width(), 54.0),
                            Sense::click(),
                        );
                        let bg = if selected { p.accent_soft } else { p.card };
                        let stroke = if selected {
                            Stroke::new(1.4, p.accent)
                        } else {
                            Stroke::new(1.0, p.border)
                        };
                        ui.painter().rect(r, CornerRadius::same(9), bg, stroke, StrokeKind::Inside);
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        let dot_c = pos2(r.right() - 18.0, r.center().y);
                        ui.painter().circle_stroke(
                            dot_c,
                            7.0,
                            Stroke::new(1.4, if selected { p.accent } else { p.border_strong }),
                        );
                        if selected {
                            ui.painter().circle_filled(dot_c, 4.0, p.accent);
                        }
                        let lr = Rect::from_min_max(
                            pos2(r.left() + 12.0, r.top() + 8.0),
                            pos2(r.right() - 34.0, r.top() + 28.0),
                        );
                        text_fit(ui, lr, label, theme::bold(13.5), p.text, Align2::RIGHT_CENTER);
                        let dr = Rect::from_min_max(
                            pos2(r.left() + 12.0, r.top() + 28.0),
                            pos2(r.right() - 34.0, r.top() + 48.0),
                        );
                        text_fit(ui, dr, desc, FontId::proportional(11.5), p.text_dim, Align2::RIGHT_CENTER);
                        if resp.clicked() {
                            st.flow = flow;
                        }
                        ui.add_space(6.0);
                    }
                });
            });
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.checkbox(
                    &mut st.auto_cascade,
                    "تجربة الطرق الأخرى تلقائياً عند فشل طريقة",
                );
            });
        });
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.checkbox(&mut st.keep_data, "الاحتفاظ ببيانات المستخدم إن أمكن");
            });
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let go =
                    egui::Button::new(RichText::new("متابعة").font(FontId::proportional(13.5)))
                        .fill(p.accent)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(110.0, 34.0));
                if ui.add(go).clicked() {
                    st.stage = 1;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("إلغاء").color(p.text_dim))
                            .fill(p.card)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(88.0, 34.0)),
                    )
                    .clicked()
                {
                    asked = Asked::Close;
                }
            });
        });
        return asked;
    }

    // ---- stage 2: final confirmation
    let danger = st.flow == Flow::Purge;
    let accent = if danger { p.danger } else { p.warn };
    let soft = if danger { p.danger_soft } else { p.warn_soft };
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(9), soft);
    let warn_rect = Rect::from_min_max(
        pos2(r.left() + 12.0, r.top() + 6.0),
        pos2(r.right() - 12.0, r.bottom() - 6.0),
    );
    let warn_text = if danger {
        "سيتم حذف التطبيق نهائياً مع ملفاته وبياناته وسجله. لا يمكن التراجع."
    } else if st.flow == Flow::FilesOnly {
        "سيتم حذف مجلد البرنامج فقط، وقد يبقى الإدخال في قائمة البرامج."
    } else {
        "سيتم تشغيل أداة الإزالة الرسمية، ثم تجربة البدائل عند الحاجة."
    };
    bidi_line(ui, warn_rect, warn_text, FontId::proportional(12.5), accent);

    ui.add_space(10.0);
    let steps = [
        ("تشغيل العمليات", "إغلاق التطبيق إن كان مفتوحاً"),
        ("نسخة احتياطية", "تصدير مفتاح السجل قبل الحذف"),
        ("نقطة استعادة", "حماية النظام قبل التعديل"),
        ("تنفيذ الحذف", "بالطريقة المختارة ثم البدائل"),
        ("التحقق", "التأكد من اختفاء الملفات"),
    ];
    for (i, (t, d)) in steps.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let c = pos2(ui.cursor().right() - 10.0, ui.cursor().top() + 9.0);
                ui.painter().circle_filled(c, 8.0, p.accent_soft);
                ui.painter().text(
                    c,
                    Align2::CENTER_CENTER,
                    format!("{}", i + 1),
                    FontId::proportional(10.0),
                    p.accent_text,
                );
                ui.add_space(22.0);
                ui.label(RichText::new(*t).font(theme::bold(12.5)).color(p.text));
                ui.label(
                    RichText::new(*d)
                        .font(FontId::proportional(11.5))
                        .color(p.text_faint),
                );
            });
        });
        ui.add_space(2.0);
    }

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let ack_text = if danger {
                "أفهم أن هذا الحذف نهائي ولا يمكن التراجع عنه"
            } else {
                "أوافق على المتابعة"
            };
            let ack_color = if st.ack { p.text } else { p.text_dim };
            ui.checkbox(&mut st.ack, RichText::new(ack_text).color(ack_color));
        });
    });

    // Permanent deletion requires typing the numeric id: a single misplaced
    // click must never be able to erase an installation, and an id can always
    // be typed even when the app name contains odd symbols.
    let typed_ok = !danger || typed_matches(st);
    if danger {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("اكتب رقم المعرّف للتأكيد")
                        .font(FontId::proportional(11.5))
                        .color(p.text_dim),
                );
                let color = if typed_ok { p.ok } else { p.text };
                ui.add(
                    egui::TextEdit::singleline(&mut st.typed)
                        .desired_width(120.0)
                        .font(FontId::proportional(13.0))
                        .text_color(color)
                        .hint_text(format!("{}", st.app.id)),
                );
                // Show the id as a selectable, copyable badge so it is always
                // readable no matter what the name looks like.
                let id_text = format!("#{}", st.app.id);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(id_text)
                                .font(FontId::monospace(12.5))
                                .color(p.accent_text),
                        )
                        .fill(p.accent_soft)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(vec2(64.0, 24.0)),
                    )
                    .on_hover_text("انسخ هذا الرقم")
                    .clicked()
                {
                    ui.ctx().copy_text(format!("{}", st.app.id));
                }
            });
        });
    }

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let armed = st.ack && typed_ok;
            let go = egui::Button::new(
                RichText::new(if danger {
                    "حذف نهائي الآن"
                } else {
                    "بدء الإزالة"
                })
                .font(FontId::proportional(13.5))
                .color(if armed { Color32::WHITE } else { p.text_faint }),
            )
            .fill(if !armed {
                p.card_hover
            } else if danger {
                p.danger
            } else {
                p.accent
            })
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(150.0, 36.0));
            if ui.add_enabled(armed, go).clicked() {
                asked = Asked::Confirm {
                    app_id: st.app.id,
                    flow: st.flow,
                    auto_cascade: st.auto_cascade,
                    keep_data: st.keep_data,
                };
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("رجوع").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(88.0, 36.0)),
                )
                .clicked()
            {
                st.stage = 0;
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("إلغاء").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(88.0, 36.0)),
                )
                .clicked()
            {
                asked = Asked::Close;
            }
        });
    });

    asked
}

// ---------------------------------------------------------------- job progress

pub fn job(ui: &mut Ui, p: &Palette, job: &Job) -> Asked {
    let mut asked = Asked::None;
    ui.set_width(620.0);

    title(ui, p, job.flow.title_ar());
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("#{}", job.app_id))
                    .font(FontId::monospace(11.0))
                    .color(p.text_faint),
            );
            ui.label(
                RichText::new(&job.app_name)
                    .font(theme::bold(13.0))
                    .color(p.text_dim),
            );
        });
    });
    ui.add_space(8.0);

    let (done, total) = job.progress();
    let frac = if total == 0 {
        0.0
    } else {
        done as f32 / total as f32
    };
    let bar = ProgressBar::new(frac)
        .desired_width(ui.available_width())
        .fill(p.accent)
        .text(RichText::new(format!("{} / {}", done, total)).color(p.text_dim));
    ui.add(bar);
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .max_height(320.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for st in &job.steps {
                let (col, label) = match st.status {
                    StepStatus::Pending => (p.text_faint, "…"),
                    StepStatus::Running => (p.accent, "▶"),
                    StepStatus::Ok => (p.ok, "✓"),
                    StepStatus::Warn => (p.warn, "!"),
                    StepStatus::Fail => (p.danger, "✕"),
                    StepStatus::Skip => (p.text_faint, "—"),
                };
                let (r, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
                let bg = if st.status == StepStatus::Running {
                    p.accent_soft
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(r, 7, bg);
                let ic = pos2(r.right() - 16.0, r.center().y);
                ui.painter()
                    .circle_filled(ic, 9.0, col.gamma_multiply(0.22));
                ui.painter().text(
                    ic,
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(10.0),
                    col,
                );
                let tr = Rect::from_min_max(
                    pos2(r.left() + 10.0, r.top() + 4.0),
                    pos2(r.right() - 32.0, r.top() + 21.0),
                );
                text_fit(
                    ui,
                    tr,
                    &st.title,
                    theme::bold(12.5),
                    p.text,
                    Align2::RIGHT_CENTER,
                );
                let dr = Rect::from_min_max(
                    pos2(r.left() + 10.0, r.top() + 20.0),
                    pos2(r.right() - 32.0, r.top() + 36.0),
                );
                let detail = &st.detail;
                // Details often embed Latin paths, so they need the bidi-aware
                // painter instead of plain left-to-right layout.
                widgets::bidi_line(ui, dr, detail, FontId::proportional(11.0), p.text_faint);
            }
        });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if job.done {
                let ok = job.success;
                let (col, bg, txt) = if ok {
                    (p.ok, p.ok_soft, "تمت العملية بنجاح")
                } else {
                    (p.warn, p.warn_soft, "انتهت العملية مع ملاحظات")
                };
                // Banner behind the result so the outcome is unmistakable.
                egui::Frame::NONE
                    .fill(bg)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.label(RichText::new(txt).font(theme::bold(13.0)).color(col));
                    });
                if ui
                    .add(
                        egui::Button::new(RichText::new("إغلاق").color(Color32::WHITE))
                            .fill(p.accent)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(96.0, 34.0)),
                    )
                    .clicked()
                {
                    asked = Asked::Close;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("إعادة الفحص").color(p.text_dim))
                            .fill(p.card)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(110.0, 34.0)),
                    )
                    .clicked()
                {
                    asked = Asked::Rescan;
                }
            } else {
                ui.label(
                    RichText::new("جارٍ التنفيذ، لا تغلق التطبيق")
                        .font(FontId::proportional(12.0))
                        .color(p.text_dim),
                );
            }
        });
    });

    asked
}

// ---------------------------------------------------------------- details

pub fn details(ui: &mut Ui, p: &Palette, app: &AppEntry, ctx: &super::widgets::Ctx) -> Asked {
    let mut asked = Asked::None;
    ui.set_width(600.0);

    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (r, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
            widgets::paint_icon(ui, ctx, app, r, 84);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(&app.name)
                            .font(theme::bold(17.0))
                            .color(p.text),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let src = if app.is_remnant() {
                        "بقايا إزالة غير مكتملة"
                    } else if app.is_system {
                        "تطبيق نظام"
                    } else {
                        app.source.label_ar()
                    };
                    ui.label(RichText::new(src).font(FontId::proportional(12.0)).color(
                        if app.is_remnant() {
                            p.danger
                        } else if app.is_system {
                            p.warn
                        } else {
                            p.text_dim
                        },
                    ));
                });
            });
        });
    });

    ui.add_space(10.0);
    // The sheet can be taller than the screen once traces and notes are
    // included, so the body scrolls and the footer stays put.
    egui::ScrollArea::vertical()
        .max_height(500.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section(ui, p, "معلومات عامة");
            field(ui, p, "المعرّف", &format!("#{}", app.id), true);
            field(ui, p, "الناشر", &app.publisher, false);
            field(ui, p, "الإصدار", &app.version, false);
            field(ui, p, "الحجم", &human_size(app.size_bytes), false);
            field(ui, p, "تاريخ التثبيت", &app.install_date, false);
            field(ui, p, "المعمارية", app.arch.text(), false);

            ui.add_space(4.0);
            section(ui, p, "الملفات");
            field(
                ui,
                p,
                "مجلد التثبيت",
                &app.install_dir
                    .as_ref()
                    .map(|d| d.to_string_lossy().to_string())
                    .unwrap_or_default(),
                true,
            );
            field(
                ui,
                p,
                "ملف التشغيل",
                &app.launch_exe
                    .as_ref()
                    .map(|d| d.to_string_lossy().to_string())
                    .unwrap_or_default(),
                true,
            );

            ui.add_space(4.0);
            section(ui, p, "الإزالة والمصدر");
            field(
                ui,
                p,
                "أداة الإزالة",
                app.uninstall_string.as_deref().unwrap_or(""),
                true,
            );
            if let Some(q) = &app.quiet_uninstall_string {
                field(ui, p, "الإزالة الصامتة", q, true);
            }
            if let Some(pkg) = &app.package_full_name {
                field(ui, p, "حزمة المتجر", pkg, true);
            }
            if let Some((root, key)) = &app.reg_key {
                field(
                    ui,
                    p,
                    "مفتاح السجل",
                    &format!("{}\\{}", root.label(), key),
                    true,
                );
            }
            field(
                ui,
                p,
                "عدد الاختصارات",
                &format!("{}", app.shortcuts.len()),
                false,
            );
            if !app.traces.is_empty() {
                field(
                    ui,
                    p,
                    "آثار إضافية",
                    &format!("{}", app.traces.len()),
                    false,
                );
                for t in app.traces.iter().take(6) {
                    let s = t.to_string_lossy().to_string();
                    field(ui, p, "أثر", &s, true);
                }
            }
            if app.source == Source::Uwp {
                field(ui, p, "النوع", "حزمة تطبيق حديثة", false);
            }

            // Free-form note the user can keep per app. Saved with the entry.
            ui.add_space(6.0);
            section(ui, p, "ملاحظات");
            let mut notes = app.notes.clone();
            ui.add(
                egui::TextEdit::multiline(&mut notes)
                    .desired_width(ui.available_width())
                    .desired_rows(2)
                    .hint_text("اكتب ملاحظة عن هذا التطبيق")
                    .font(FontId::proportional(12.5)),
            );
            if notes != app.notes {
                asked = Asked::SetNotes {
                    app_id: app.id,
                    notes,
                };
            }

            // Elevation state matters here: a non-admin run may fail to delete.
            if !ctx.elevated {
                ui.add_space(4.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("بدون صلاحيات مدير")
                            .font(FontId::proportional(11.5))
                            .color(p.warn),
                    );
                });
            }
        });

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Protected folders must not offer a delete button that looks live.
            let blocked = app.blocked_reason();
            if ui
                .add(
                    egui::Button::new(RichText::new("حذف كامل").color(if blocked.is_some() {
                        p.text_faint
                    } else {
                        p.danger
                    }))
                    .fill(if blocked.is_some() {
                        p.card
                    } else {
                        p.danger_soft
                    })
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(100.0, 34.0)),
                )
                .on_hover_text(blocked.unwrap_or("حذف نهائي مع كل الجذور والبقايا"))
                .clicked()
            {
                asked = Asked::Confirm {
                    app_id: app.id,
                    flow: Flow::Purge,
                    auto_cascade: true,
                    keep_data: false,
                };
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("إزالة").color(if blocked.is_some() {
                        p.text_faint
                    } else {
                        p.accent_text
                    }))
                    .fill(if blocked.is_some() {
                        p.card
                    } else {
                        p.accent_soft
                    })
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(88.0, 34.0)),
                )
                .on_hover_text(blocked.unwrap_or("تشغيل أداة الإزالة مع بدائل تلقائية"))
                .clicked()
            {
                asked = Asked::Confirm {
                    app_id: app.id,
                    flow: Flow::Uninstall,
                    auto_cascade: true,
                    keep_data: true,
                };
            }
            if app.launch_exe.is_some()
                && ui
                    .add(
                        egui::Button::new(RichText::new("تشغيل").color(p.ok))
                            .fill(p.card)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(80.0, 34.0)),
                    )
                    .clicked()
            {
                asked = Asked::Run(app.id);
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("فتح المجلد").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(104.0, 34.0)),
                )
                .clicked()
            {
                asked = Asked::OpenFolder(app.id);
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("نسخ المسار").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(104.0, 34.0)),
                )
                .clicked()
            {
                asked = Asked::CopyPath(app.id);
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("إغلاق").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(80.0, 34.0)),
                )
                .clicked()
            {
                asked = Asked::Close;
            }
        });
    });

    asked
}

// ---------------------------------------------------------------- settings

pub fn settings(ui: &mut Ui, p: &Palette, s: &mut Settings) -> (Asked, bool) {
    let mut asked = Asked::None;
    let mut changed = false;
    ui.set_width(720.0);
    title(ui, p, "الإعدادات");

    egui::ScrollArea::vertical()
        .max_height(560.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            section(ui, p, "المظهر");
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("الوضع").color(p.text_dim));
                    for (m, l) in [(ThemeMode::Light, "فاتح"), (ThemeMode::Dark, "داكن")] {
                        let sel = s.theme == m;
                        if ui
                            .add(
                                egui::Button::new(RichText::new(l).color(if sel {
                                    p.accent_text
                                } else {
                                    p.text_dim
                                }))
                                .fill(if sel { p.accent_soft } else { p.card })
                                .corner_radius(CornerRadius::same(8))
                                .min_size(vec2(72.0, 30.0)),
                            )
                            .clicked()
                        {
                            s.theme = m;
                            changed = true;
                        }
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("لون التمييز").color(p.text_dim));
                    for a in Accent::ALL {
                        let (r, g, b) = a.rgb();
                        let col = Color32::from_rgb(r, g, b);
                        let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
                        ui.painter().circle_filled(rect.center(), 12.0, col);
                        if s.accent == a {
                            ui.painter().circle_stroke(
                                rect.center(),
                                14.5,
                                Stroke::new(1.6, p.text),
                            );
                        }
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            s.accent = a;
                            changed = true;
                        }
                        resp.on_hover_text(a.label_ar());
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("كثافة العرض").color(p.text_dim));
                    for d in [Density::Compact, Density::Normal, Density::Relaxed] {
                        let sel = s.density == d;
                        if ui
                            .add(
                                egui::Button::new(RichText::new(d.label_ar()).color(if sel {
                                    p.accent_text
                                } else {
                                    p.text_dim
                                }))
                                .fill(if sel { p.accent_soft } else { p.card })
                                .corner_radius(CornerRadius::same(8))
                                .min_size(vec2(64.0, 30.0)),
                            )
                            .clicked()
                        {
                            s.density = d;
                            changed = true;
                        }
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("طريقة العرض").color(p.text_dim));
                    for (v, l) in [(ViewMode::Cards, "كروت"), (ViewMode::Table, "جدول")] {
                        let sel = s.view == v;
                        if ui
                            .add(
                                egui::Button::new(RichText::new(l).color(if sel {
                                    p.accent_text
                                } else {
                                    p.text_dim
                                }))
                                .fill(if sel { p.accent_soft } else { p.card })
                                .corner_radius(CornerRadius::same(8))
                                .min_size(vec2(72.0, 30.0)),
                            )
                            .clicked()
                        {
                            s.view = v;
                            changed = true;
                        }
                    }
                });
            });

            if s.view == ViewMode::Cards {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add(
                            egui::Slider::new(&mut s.card_min_width, 220.0..=460.0)
                                .text("عرض الكرت"),
                        );
                    });
                });
            }
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(egui::Slider::new(&mut s.icon_size, 28.0..=96.0).text("حجم الأيقونة"));
                });
            });

            section(ui, p, "ما يظهر في القائمة");
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.show_system, "إظهار تطبيقات النظام");
                });
            });
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.show_frameworks, "إظهار مكونات التشغيل المشتركة");
                });
            });
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.show_shortcuts, "إظهار الاختصارات غير المرتبطة");
                });
            });

            section(ui, p, "سلوك الحذف");
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.auto_cascade, "تجربة كل الطرق تلقائياً عند الفشل");
                });
            });
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.create_restore_point, "إنشاء نقطة استعادة قبل الحذف");
                });
            });
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.backup_registry, "نسخ مفتاح السجل احتياطياً");
                });
            });
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.kill_processes, "إغلاق التطبيق قبل الحذف");
                });
            });

            section(ui, p, "عام");
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut s.scan_on_start, "فحص النظام عند التشغيل");
                });
            });
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("الترتيب").color(p.text_dim));
                    egui::ComboBox::from_id_salt("sortkey")
                        .selected_text(s.sort.label_ar())
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for k in [
                                SortKey::Name,
                                SortKey::Publisher,
                                SortKey::Size,
                                SortKey::Date,
                                SortKey::Source,
                            ] {
                                if ui.selectable_value(&mut s.sort, k, k.label_ar()).clicked() {
                                    changed = true;
                                }
                            }
                        });
                    ui.checkbox(&mut s.sort_desc, "تنازلي");
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "الإعدادات محفوظة في: {}",
                            Settings::path().to_string_lossy()
                        ))
                        .font(FontId::monospace(10.5))
                        .color(p.text_faint),
                    );
                });
            });
        });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("حفظ وإغلاق").color(Color32::WHITE))
                        .fill(p.accent)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(130.0, 34.0)),
                )
                .clicked()
            {
                asked = Asked::Close;
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("إعادة ضبط").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(110.0, 34.0)),
                )
                .clicked()
            {
                *s = Settings::default();
                changed = true;
            }
        });
    });

    (asked, changed)
}

/// Wrap content in a modal with the app's palette.
pub fn show_modal<R>(
    ctx: &egui::Context,
    p: &Palette,
    id: &str,
    content: impl FnOnce(&mut Ui) -> R,
) -> (R, bool) {
    let m = Modal::new(Id::unique(id))
        .frame(modal_frame(p))
        .backdrop_color(Color32::from_black_alpha(if p.dark { 170 } else { 90 }))
        .show(ctx, content);
    let close = m.backdrop_response.clicked() && !m.any_popup_open;
    (m.inner, close)
}

/// Small utility used by the header: a pill button with hover tint.
pub fn nav_pill(ui: &mut Ui, p: &Palette, label: &str, active: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(
        vec2(
            ui.painter()
                .layout_no_wrap(label.to_string(), FontId::proportional(12.5), p.text)
                .size()
                .x
                + 26.0,
            30.0,
        ),
        Sense::click(),
    );
    let bg = if active {
        p.accent_soft
    } else if resp.hovered() {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(r, CornerRadius::same(8), bg);
    if active {
        ui.painter().rect_stroke(
            r,
            CornerRadius::same(8),
            Stroke::new(1.0, p.accent),
            StrokeKind::Inside,
        );
    }
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.5),
        if active { p.accent_text } else { p.text_dim },
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// Re-exported so the module can build unused-import warnings away.
#[allow(dead_code)]
fn _unused(_v: Vec2, _w: &dyn Fn(&mut Ui)) {
    let _ = widgets::is_arabic('ا');
}

// ---------------------------------------------------------------- backups

/// What the backup manager asked for.
pub enum BackupAct {
    None,
    Delete(String),
    ClearAll,
    RevealFolder,
    Import(String),
}

/// Lists saved registry backups with per-item and bulk delete.
/// `confirm_clear` makes the bulk-delete button ask for a second click.
pub fn backups(
    ui: &mut Ui,
    p: &Palette,
    items: &[crate::actions::BackupEntry],
    confirm_clear: &mut bool,
) -> BackupAct {
    let mut act = BackupAct::None;
    ui.set_width(680.0);

    title(ui, p, "النسخ المحفوظة");
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let total = crate::actions::backups_size();
            // Never concatenate digits into an Arabic string: epaint has no
            // bidi reordering, so each part gets its own widget. Widgets are
            // added right-to-left, so the number comes before its noun.
            ui.label(
                RichText::new(format!("{}", items.len()))
                    .font(FontId::monospace(11.5))
                    .color(p.text_faint),
            );
            ui.label(
                RichText::new("ملف محفوظ")
                    .font(FontId::proportional(11.5))
                    .color(p.text_faint),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(human_size(total))
                    .font(FontId::monospace(11.5))
                    .color(p.text_faint),
            );
            ui.label(
                RichText::new("الحجم الكلي")
                    .font(FontId::proportional(11.5))
                    .color(p.text_faint),
            );
        });
    });

    ui.add_space(8.0);

    if items.is_empty() {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
        ui.painter().rect(
            r,
            CornerRadius::same(10),
            p.card,
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            "لا توجد نسخ محفوظة",
            FontId::proportional(13.0),
            p.text_dim,
        );
    } else {
        let list_h = (items.len() as f32 * 46.0).min(300.0);
        egui::ScrollArea::vertical()
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for b in items {
                    let (r, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
                    ui.painter().rect(
                        r,
                        CornerRadius::same(9),
                        p.card,
                        Stroke::new(1.0, p.border),
                        StrokeKind::Inside,
                    );

                    // Name (Latin, left of the row) and metadata.
                    let name_r = Rect::from_min_max(
                        pos2(r.left() + 150.0, r.top() + 4.0),
                        pos2(r.right() - 12.0, r.top() + 21.0),
                    );
                    widgets::text_fit(
                        ui,
                        name_r,
                        &b.file,
                        FontId::proportional(12.0),
                        p.text,
                        Align2::RIGHT_CENTER,
                    );
                    let meta_r = Rect::from_min_max(
                        pos2(r.left() + 150.0, r.top() + 20.0),
                        pos2(r.right() - 12.0, r.top() + 37.0),
                    );
                    widgets::text_fit(
                        ui,
                        meta_r,
                        &format!("{}  ·  {}", b.modified, human_size(b.size)),
                        FontId::proportional(10.5),
                        p.text_faint,
                        Align2::RIGHT_CENTER,
                    );

                    let del = Rect::from_min_size(
                        pos2(r.left() + 10.0, r.center().y - 12.0),
                        vec2(58.0, 24.0),
                    );
                    if widgets::pill(
                        ui,
                        ("delbk", b.path.to_string_lossy().to_string()),
                        del,
                        "حذف",
                        p.danger,
                        p.danger_soft,
                        p.danger,
                        11.5,
                    )
                    .clicked()
                    {
                        act = BackupAct::Delete(b.path.to_string_lossy().to_string());
                    }

                    let imp = Rect::from_min_size(
                        pos2(r.left() + 76.0, r.center().y - 12.0),
                        vec2(64.0, 24.0),
                    );
                    if widgets::pill(
                        ui,
                        ("impbk", b.path.to_string_lossy().to_string()),
                        imp,
                        "استعادة",
                        p.accent_text,
                        p.accent_soft,
                        p.accent,
                        11.5,
                    )
                    .on_hover_text("استيراد هذا الملف إلى السجل")
                    .clicked()
                    {
                        act = BackupAct::Import(b.path.to_string_lossy().to_string());
                    }
                }
            });
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("إغلاق").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(80.0, 34.0)),
                )
                .clicked()
            {
                act = BackupAct::None;
                ui.close();
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("فتح المجلد").color(p.text_dim))
                        .fill(p.card)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(104.0, 34.0)),
                )
                .clicked()
            {
                act = BackupAct::RevealFolder;
            }
            if !items.is_empty() {
                let label = if *confirm_clear {
                    "تأكيد حذف الكل"
                } else {
                    "حذف كل النسخ"
                };
                let fill = if *confirm_clear {
                    p.danger
                } else {
                    p.danger_soft
                };
                let fg = if *confirm_clear {
                    Color32::WHITE
                } else {
                    p.danger
                };
                if ui
                    .add(
                        egui::Button::new(RichText::new(label).color(fg))
                            .fill(fill)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(130.0, 34.0)),
                    )
                    .clicked()
                {
                    if *confirm_clear {
                        *confirm_clear = false;
                        act = BackupAct::ClearAll;
                    } else {
                        *confirm_clear = true;
                    }
                }
            }
        });
    });

    act
}
