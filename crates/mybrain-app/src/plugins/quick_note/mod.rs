//! Quick Notes view, Google Keep-style cards board, and quick capture composer plugin.

use chrono::Local;
use egui::{
    vec2, Color32, CornerRadius, Frame, Id, Margin, RichText, ScrollArea, Stroke, Ui,
};
use mybrain_core::frontmatter::Frontmatter;
use mybrain_core::index::LinkIndex;
use mybrain_core::quick::QuickNotes;
use mybrain_core::vault::Vault;
use mybrain_core::{Note, NoteId};

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::{self, Tokens};

pub struct QuickColorDef {
    pub name: &'static str,
    pub label: &'static str,
    pub dark_bg: Color32,
    pub light_bg: Color32,
    pub dot_color: Color32,
}

pub const QUICK_COLORS: &[QuickColorDef] = &[
    QuickColorDef {
        name: "default",
        label: "Default",
        dark_bg: Color32::from_rgb(28, 29, 34),
        light_bg: Color32::from_rgb(255, 255, 255),
        dot_color: Color32::from_rgb(140, 140, 145),
    },
    QuickColorDef {
        name: "coral",
        label: "Coral",
        dark_bg: Color32::from_rgb(95, 36, 28),
        light_bg: Color32::from_rgb(250, 222, 218),
        dot_color: Color32::from_rgb(235, 87, 87),
    },
    QuickColorDef {
        name: "peach",
        label: "Peach",
        dark_bg: Color32::from_rgb(90, 48, 22),
        light_bg: Color32::from_rgb(254, 239, 219),
        dot_color: Color32::from_rgb(242, 153, 74),
    },
    QuickColorDef {
        name: "sand",
        label: "Sand",
        dark_bg: Color32::from_rgb(85, 65, 20),
        light_bg: Color32::from_rgb(255, 248, 184),
        dot_color: Color32::from_rgb(242, 201, 76),
    },
    QuickColorDef {
        name: "mint",
        label: "Mint",
        dark_bg: Color32::from_rgb(28, 70, 44),
        light_bg: Color32::from_rgb(226, 246, 211),
        dot_color: Color32::from_rgb(39, 174, 96),
    },
    QuickColorDef {
        name: "teal",
        label: "Teal",
        dark_bg: Color32::from_rgb(22, 68, 72),
        light_bg: Color32::from_rgb(212, 244, 240),
        dot_color: Color32::from_rgb(45, 156, 219),
    },
    QuickColorDef {
        name: "sky",
        label: "Sky Blue",
        dark_bg: Color32::from_rgb(25, 55, 86),
        light_bg: Color32::from_rgb(218, 235, 252),
        dot_color: Color32::from_rgb(86, 204, 242),
    },
    QuickColorDef {
        name: "dusk",
        label: "Dusk",
        dark_bg: Color32::from_rgb(38, 44, 82),
        light_bg: Color32::from_rgb(222, 227, 251),
        dot_color: Color32::from_rgb(111, 82, 216),
    },
    QuickColorDef {
        name: "lavender",
        label: "Lavender",
        dark_bg: Color32::from_rgb(58, 36, 80),
        light_bg: Color32::from_rgb(241, 224, 251),
        dot_color: Color32::from_rgb(155, 81, 224),
    },
    QuickColorDef {
        name: "pink",
        label: "Blossom",
        dark_bg: Color32::from_rgb(76, 32, 58),
        light_bg: Color32::from_rgb(253, 222, 238),
        dot_color: Color32::from_rgb(235, 87, 137),
    },
    QuickColorDef {
        name: "clay",
        label: "Clay",
        dark_bg: Color32::from_rgb(54, 40, 32),
        light_bg: Color32::from_rgb(230, 201, 180),
        dot_color: Color32::from_rgb(180, 130, 100),
    },
];

pub fn card_bg_for_color(color_name: Option<&str>, is_dark: bool, default_bg: Color32) -> Color32 {
    if let Some(c) = color_name {
        if let Some(def) = QUICK_COLORS.iter().find(|d| d.name == c) {
            return if is_dark { def.dark_bg } else { def.light_bg };
        }
        if let Some(parsed) = theme::parse_color_str(c) {
            return if is_dark {
                parsed.gamma_multiply(0.35)
            } else {
                parsed.gamma_multiply(0.2)
            };
        }
    }
    default_bg
}

pub struct QuickNotePlugin {
    pub composer_title: String,
    pub composer_body: String,
    pub composer_expanded: bool,
    pub composer_pinned: bool,
    pub composer_color: Option<String>,
    pub search_query: String,

    // In-place Keep note edit dialog
    pub edit_note: Option<NoteId>,
    pub edit_title: String,
    pub edit_body: String,
    pub edit_pinned: bool,
    pub edit_color: Option<String>,

    // Quick capture floating modal
    pub show_quick_capture_modal: bool,
    pub quick_capture_buffer: String,
}

impl QuickNotePlugin {
    pub fn new() -> Self {
        Self {
            composer_title: String::new(),
            composer_body: String::new(),
            composer_expanded: false,
            composer_pinned: false,
            composer_color: None,
            search_query: String::new(),
            edit_note: None,
            edit_title: String::new(),
            edit_body: String::new(),
            edit_pinned: false,
            edit_color: None,
            show_quick_capture_modal: false,
            quick_capture_buffer: String::new(),
        }
    }

    pub fn submit_composer(&mut self, vault: &mut Vault) -> Option<NoteId> {
        let text = self.composer_body.trim();
        let title = self.composer_title.trim();
        if text.is_empty() && title.is_empty() {
            self.composer_expanded = false;
            return None;
        }

        let clean_title = if title.is_empty() {
            let first_line = text.lines().next().unwrap_or("Quick Note");
            if first_line.len() > 30 {
                format!("{}…", &first_line[..30])
            } else {
                first_line.to_string()
            }
        } else {
            title.to_string()
        };

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let safe_name = clean_title
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        let id_str = format!("Quick Notes/{timestamp}_{safe_name}");
        let note_id = NoteId::new(&id_str).unwrap_or_else(|_| NoteId::new(&format!("Quick Notes/{timestamp}")).unwrap());

        let mut content = String::new();
        let has_fm = self.composer_pinned || self.composer_color.is_some();
        if has_fm {
            content.push_str("---\n");
            if self.composer_pinned {
                content.push_str("pinned: true\n");
            }
            if let Some(ref c) = self.composer_color {
                content.push_str(&format!("color: {c}\n"));
            }
            content.push_str("---\n\n");
        }
        if !title.is_empty() {
            content.push_str(&format!("# {title}\n\n"));
        }
        content.push_str(text);
        if !content.ends_with('\n') {
            content.push('\n');
        }

        let _ = vault.create(note_id.clone(), &content);

        self.composer_title.clear();
        self.composer_body.clear();
        self.composer_pinned = false;
        self.composer_color = None;
        self.composer_expanded = false;

        Some(note_id)
    }
}

impl Default for QuickNotePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for QuickNotePlugin {
    fn id(&self) -> &'static str {
        "core.quick_note"
    }

    fn name(&self) -> &'static str {
        "Quick Notes"
    }

    fn description(&self) -> &'static str {
        "Google Keep-style cards board and quick capture for rapid thought and idea collection"
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "⚡",
            tooltip: "Quick Notes (Ctrl+Q)",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::MainWorkspace,
            priority: 30,
        })
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "open_quick_notes".into(),
                title: "Open Quick Notes Board".into(),
                shortcut: None,
                plugin_id: "core.quick_note",
            },
            PluginCommand {
                id: "quick_capture".into(),
                title: "Quick Capture Note".into(),
                shortcut: Some("Ctrl+Q".into()),
                plugin_id: "core.quick_note",
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        match command_id {
            "open_quick_notes" => {
                ctx.activate_plugin("core.quick_note");
            }
            "quick_capture" => {
                self.show_quick_capture_modal = true;
                self.quick_capture_buffer.clear();
            }
            _ => {}
        }
    }

    fn render_main_view(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());
        let is_dark = ctx.theme_kind.is_dark();

        let mut note_to_open = None;
        let mut note_to_edit = None;
        let mut note_to_delete = None;
        let mut toggle_pin = None;

        ScrollArea::vertical()
            .id_salt("quick_notes_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::NONE
                    .inner_margin(Margin::symmetric(24, 16))
                    .show(ui, |ui| {
                        let avail_w = ui.available_width();
                        let max_w = avail_w.min(theme::READABLE_WIDTH + 140.0);
                        let side_pad = ((avail_w - max_w) * 0.5).max(0.0);

                        ui.horizontal(|ui| {
                            if side_pad > 0.0 {
                                ui.add_space(side_pad);
                            }
                            ui.vertical(|ui| {
                                ui.set_max_width(max_w);

                                // 1. Top Quick Note Composer
                                let comp_bg = card_bg_for_color(self.composer_color.as_deref(), is_dark, t.card);
                                Frame::NONE
                                    .fill(comp_bg)
                                    .stroke(Stroke::new(1.0, t.card_border))
                                    .corner_radius(CornerRadius::same(t.radius as u8))
                                    .inner_margin(Margin::same(14))
                                    .shadow(egui::Shadow {
                                        offset: [0, 4],
                                        blur: 16,
                                        spread: 0,
                                        color: t.shadow,
                                    })
                                    .show(ui, |ui| {
                                        if !self.composer_expanded {
                                            let resp = ui.add(
                                                egui::TextEdit::singleline(&mut self.composer_body)
                                                    .hint_text("Take a note… (click to expand)")
                                                    .frame(Frame::NONE)
                                                    .desired_width(ui.available_width()),
                                            );
                                            if resp.gained_focus() || resp.clicked() {
                                                self.composer_expanded = true;
                                            }
                                        } else {
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::TextEdit::singleline(&mut self.composer_title)
                                                        .hint_text("Title")
                                                        .frame(Frame::NONE)
                                                        .desired_width(ui.available_width() - 36.0),
                                                );
                                                let pin_col = if self.composer_pinned { t.accent_text } else { t.text_faint };
                                                if ui.button(RichText::new("📌").color(pin_col).size(13.0)).clicked() {
                                                    self.composer_pinned = !self.composer_pinned;
                                                }
                                            });

                                            ui.add_space(8.0);
                                            ui.add(
                                                egui::TextEdit::multiline(&mut self.composer_body)
                                                    .hint_text("Take a note…")
                                                    .frame(Frame::NONE)
                                                    .desired_rows(3)
                                                    .desired_width(ui.available_width()),
                                            );

                                            ui.add_space(10.0);
                                            ui.horizontal(|ui| {
                                                for col in QUICK_COLORS.iter().take(6) {
                                                    let is_sel = self.composer_color.as_deref() == Some(col.name);
                                                    let btn = egui::Button::new(" ")
                                                        .fill(col.dot_color)
                                                        .stroke(if is_sel { Stroke::new(2.0, Color32::WHITE) } else { Stroke::NONE })
                                                        .corner_radius(CornerRadius::same(4))
                                                        .min_size(vec2(16.0, 16.0));
                                                    if ui.add(btn).clicked() {
                                                        self.composer_color = if is_sel { None } else { Some(col.name.to_string()) };
                                                    }
                                                }

                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.button(RichText::new("Close").strong().color(t.accent_text)).clicked() {
                                                        self.submit_composer(ctx.vault);
                                                    }
                                                });
                                            });
                                        }
                                    });

                                ui.add_space(20.0);

                                // 2. Search filter
                                ui.horizontal(|ui| {
                                    theme::icon_search(ui, 13.0, t.text_dim);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.search_query)
                                            .hint_text("Search quick notes…")
                                            .frame(Frame::NONE)
                                            .desired_width(ui.available_width()),
                                    );
                                });
                                ui.add_space(8.0);
                                theme::bottom_hairline(ui, ui.max_rect(), &t);
                                ui.add_space(16.0);

                                // 3. Collect and Partition Notes
                                let query = self.search_query.trim().to_lowercase();
                                let mut quick_notes: Vec<Note> = ctx.vault
                                    .notes()
                                    .filter(|n| n.id.folder() == Some("Quick Notes") || n.id.as_str().starts_with("Quick Notes/"))
                                    .filter(|n| {
                                        if query.is_empty() {
                                            true
                                        } else {
                                            n.id.name().to_lowercase().contains(&query) || n.content.to_lowercase().contains(&query)
                                        }
                                    })
                                    .cloned()
                                    .collect();

                                quick_notes.sort_by(|a, b| b.id.as_str().cmp(a.id.as_str()));

                                if quick_notes.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(40.0);
                                        ui.label(RichText::new("No quick notes yet").color(t.text_dim).size(15.0).strong());
                                        ui.add_space(6.0);
                                        ui.label(RichText::new("Use the composer above to jot down quick notes, ideas, and reminders.").color(t.text_faint).size(12.5));
                                    });
                                } else {
                                    let mut pinned = Vec::new();
                                    let mut others = Vec::new();
                                    for n in quick_notes {
                                        if n.frontmatter().get("pinned").map_or(false, |v| v == "true") {
                                            pinned.push(n);
                                        } else {
                                            others.push(n);
                                        }
                                    }

                                    let mut render_notes_grid = |ui: &mut Ui, notes: &[Note], t: &Tokens| {
                                        let col_w = 230.0;
                                        ui.horizontal_wrapped(|ui| {
                                            ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                                            for note in notes {
                                                let fm = note.frontmatter();
                                                let col_name = fm.get("color");
                                                let bg = card_bg_for_color(col_name, is_dark, t.card);
                                                let is_pinned = fm.get("pinned").map_or(false, |v| v == "true");

                                                Frame::NONE
                                                    .fill(bg)
                                                    .stroke(Stroke::new(1.0, t.card_border))
                                                    .corner_radius(CornerRadius::same(t.radius as u8))
                                                    .inner_margin(Margin::same(12))
                                                    .show(ui, |ui| {
                                                        ui.set_width(col_w);
                                                        ui.horizontal(|ui| {
                                                            let title = note.title();
                                                            ui.label(RichText::new(title).strong().size(13.5).color(t.text));
                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                if ui.button(RichText::new(if is_pinned { "📌" } else { "📍" }).size(11.0)).clicked() {
                                                                    toggle_pin = Some(note.id.clone());
                                                                }
                                                            });
                                                        });
                                                        ui.add_space(4.0);
                                                        let preview = note.body().lines().take(4).collect::<Vec<_>>().join("\n");
                                                        ui.label(RichText::new(preview).color(t.text_dim).size(12.0));

                                                        ui.add_space(8.0);
                                                        ui.horizontal(|ui| {
                                                            if theme::ghost_button(ui, t, RichText::new("Edit").size(11.0).color(t.accent_text)).clicked() {
                                                                note_to_edit = Some(note.id.clone());
                                                            }
                                                            if theme::ghost_button(ui, t, RichText::new("Open ↗").size(11.0).color(t.text_dim)).clicked() {
                                                                note_to_open = Some(note.id.clone());
                                                            }
                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                if theme::ghost_button(ui, t, RichText::new("🗑").size(11.0).color(t.danger)).clicked() {
                                                                    note_to_delete = Some(note.id.clone());
                                                                }
                                                            });
                                                        });
                                                    });
                                            }
                                        });
                                    };

                                    if !pinned.is_empty() {
                                        theme::section_label(ui, &t, "PINNED");
                                        ui.add_space(4.0);
                                        render_notes_grid(ui, &pinned, &t);
                                        ui.add_space(16.0);
                                        if !others.is_empty() {
                                            theme::section_label(ui, &t, "OTHERS");
                                            ui.add_space(4.0);
                                        }
                                    }

                                    if !others.is_empty() {
                                        render_notes_grid(ui, &others, &t);
                                    }
                                }
                            });
                        });
                        ui.add_space(24.0);
                    });
            });

        if let Some(id) = note_to_open {
            ctx.open_note(id);
            ctx.activate_plugin("core.markdown");
        }

        if let Some(id) = note_to_edit {
            if let Some(n) = ctx.vault.get(&id) {
                self.edit_note = Some(id);
                self.edit_title = n.title().to_string();
                self.edit_body = n.body().to_string();
                let fm = n.frontmatter();
                self.edit_pinned = fm.get("pinned").map_or(false, |v| v == "true");
                self.edit_color = fm.get("color").map(ToString::to_string);
            }
        }

        if let Some(id) = note_to_delete {
            let _ = ctx.vault.delete(&id);
            ctx.toast("Deleted quick note");
        }

        if let Some(id) = toggle_pin {
            if let Some(n) = ctx.vault.get(&id) {
                let (mut fm, body) = Frontmatter::parse(&n.content);
                let current = fm.get("pinned").map_or(false, |v| v == "true");
                if current {
                    fm.remove("pinned");
                } else {
                    fm.set("pinned", "true");
                }
                let new_content = if fm.is_empty() {
                    body.trim_start_matches('\n').to_string()
                } else {
                    format!("{}{}", fm.render(), body.trim_start_matches('\n'))
                };
                let _ = ctx.vault.save(&id, &new_content);
                *ctx.index = LinkIndex::build(ctx.vault);
            }
        }
    }

    fn render_modal(&mut self, ctx_egui: &egui::Context, ctx: &mut PluginContext) {
        let t = Tokens::get(ctx_egui);

        // 1. Floating Quick Capture Modal
        if self.show_quick_capture_modal {
            let modal = egui::Modal::new(Id::unique("floating_quick_capture_modal"))
                .backdrop_color(Color32::from_black_alpha(150))
                .frame(theme::modal_frame(&t));

            let mut close_modal = false;
            let mut save_capture = false;

            let modal_resp = modal.show(ctx_egui, |ui| {
                ui.set_width(460.0);
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("⚡ Quick Capture").color(t.text).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon_close_button(ui, &t).clicked() {
                            close_modal = true;
                        }
                    });
                });

                ui.add_space(8.0);
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut self.quick_capture_buffer)
                        .hint_text("Capture a thought, link, or task…")
                        .desired_rows(5)
                        .frame(Frame::NONE)
                        .desired_width(ui.available_width()),
                );
                edit.request_focus();

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Press Ctrl+Enter to save").color(t.text_faint).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Save Note").strong().color(Color32::WHITE)).clicked() {
                            save_capture = true;
                        }
                    });
                });
            });

            if ctx_egui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter)) {
                save_capture = true;
            }

            if save_capture {
                if !self.quick_capture_buffer.trim().is_empty() {
                    let quick = QuickNotes {
                        folder: "Quick Notes".into(),
                        log_to_daily: true,
                    };
                    let daily = mybrain_core::daily::DailyNotes {
                        folder: ctx.settings.daily_folder.clone(),
                        ..Default::default()
                    };
                    let now = Local::now().naive_local();
                    let _ = quick.capture(ctx.vault, &daily, &self.quick_capture_buffer, now);
                    *ctx.index = LinkIndex::build(ctx.vault);
                    ctx.toast("Captured quick note");
                }
                close_modal = true;
            }

            if modal_resp.should_close() || close_modal {
                self.show_quick_capture_modal = false;
                self.quick_capture_buffer.clear();
            }
        }

        // 2. In-place Quick Note Edit Modal
        if let Some(target_id) = self.edit_note.clone() {
            let modal = egui::Modal::new(Id::unique("edit_quick_note_modal"))
                .backdrop_color(Color32::from_black_alpha(150))
                .frame(theme::modal_frame(&t));

            let mut close_edit = false;
            let mut save_edit = false;

            let modal_resp = modal.show(ctx_egui, |ui| {
                ui.set_width(480.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.edit_title)
                            .hint_text("Title")
                            .frame(Frame::NONE)
                            .desired_width(ui.available_width() - 40.0),
                    );
                    let pin_col = if self.edit_pinned { t.accent_text } else { t.text_faint };
                    if ui.button(RichText::new("📌").color(pin_col).size(13.0)).clicked() {
                        self.edit_pinned = !self.edit_pinned;
                    }
                });

                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.edit_body)
                        .hint_text("Note body…")
                        .desired_rows(6)
                        .frame(Frame::NONE)
                        .desired_width(ui.available_width()),
                );

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    for col in QUICK_COLORS.iter().take(6) {
                        let is_sel = self.edit_color.as_deref() == Some(col.name);
                        let btn = egui::Button::new(" ")
                            .fill(col.dot_color)
                            .stroke(if is_sel { Stroke::new(2.0, Color32::WHITE) } else { Stroke::NONE })
                            .corner_radius(CornerRadius::same(4))
                            .min_size(vec2(16.0, 16.0));
                        if ui.add(btn).clicked() {
                            self.edit_color = if is_sel { None } else { Some(col.name.to_string()) };
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Done").strong().color(t.accent_text)).clicked() {
                            save_edit = true;
                        }
                    });
                });
            });

            if save_edit {
                if let Some(note) = ctx.vault.get(&target_id) {
                    let (mut fm, _old_body) = Frontmatter::parse(&note.content);
                    if self.edit_pinned {
                        fm.set("pinned", "true");
                    } else {
                        fm.remove("pinned");
                    }
                    if let Some(ref c) = self.edit_color {
                        fm.set("color", c.trim());
                    } else {
                        fm.remove("color");
                    }
                    let body = format!("# {}\n\n{}", self.edit_title, self.edit_body);
                    let new_content = if fm.is_empty() {
                        body
                    } else {
                        format!("{}{}", fm.render(), body)
                    };
                    let _ = ctx.vault.save(&target_id, &new_content);
                    *ctx.index = LinkIndex::build(ctx.vault);
                    ctx.toast("Updated quick note");
                }
                close_edit = true;
            }

            if modal_resp.should_close() || close_edit {
                self.edit_note = None;
            }
        }
    }
}
