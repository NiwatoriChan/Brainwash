//! Modal dialogs for creating notes, folders, canvases, events, renaming, and color picking.

use egui::{vec2, Color32, Id, RichText};
use mybrain_core::events::EventDraft;
use mybrain_core::events_bus::AppEvent;
use mybrain_core::frontmatter::Frontmatter;
use mybrain_core::index::LinkIndex;
use mybrain_core::note::NoteId;
use mybrain_core::settings::ColorDef;

use crate::plugins::PluginContext;
use crate::theme::{self, Tokens};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomColorTarget {
    Note(NoteId),
    Folder(String),
    NewFolderDialog,
    NewNoteDialog,
}

pub struct DialogsState {
    // New Note
    pub show_new_note: bool,
    pub new_note_name: String,
    pub new_note_folder: Option<String>,
    pub new_note_emoji: Option<String>,
    pub new_note_color: Option<String>,

    // Rename Note
    pub show_rename_note: bool,
    pub rename_target: Option<NoteId>,
    pub rename_new_name: String,

    // New Folder
    pub show_new_folder: bool,
    pub new_folder_name: String,
    pub new_folder_parent: Option<String>,
    pub new_folder_emoji: Option<String>,
    pub new_folder_color: Option<String>,

    // New Canvas
    pub show_new_canvas: bool,
    pub new_canvas_name: String,
    pub new_canvas_folder: Option<String>,

    // New Event
    pub show_new_event: bool,
    pub new_event_draft: EventDraft,

    // Custom Color Dialog
    pub custom_color_target: Option<CustomColorTarget>,
    pub custom_color_hex: String,
    pub custom_color_name: String,
}

impl Default for DialogsState {
    fn default() -> Self {
        Self {
            show_new_note: false,
            new_note_name: String::new(),
            new_note_folder: None,
            new_note_emoji: None,
            new_note_color: None,

            show_rename_note: false,
            rename_target: None,
            rename_new_name: String::new(),

            show_new_folder: false,
            new_folder_name: String::new(),
            new_folder_parent: None,
            new_folder_emoji: None,
            new_folder_color: None,

            show_new_canvas: false,
            new_canvas_name: String::new(),
            new_canvas_folder: None,

            show_new_event: false,
            new_event_draft: EventDraft::new(chrono::Local::now().date_naive()),

            custom_color_target: None,
            custom_color_hex: "#2d9cdb".into(),
            custom_color_name: String::new(),
        }
    }
}

pub fn render_dialogs(
    ctx_egui: &egui::Context,
    state: &mut DialogsState,
    ctx: &mut PluginContext,
) {
    let t = Tokens::get(ctx_egui);

    // 1. New Note Dialog
    if state.show_new_note {
        let modal = egui::Modal::new(Id::unique("new_note_modal"))
            .backdrop_color(Color32::from_black_alpha(150))
            .frame(theme::modal_frame(&t));

        let mut close = false;
        let mut create_note = false;

        let modal_resp = modal.show(ctx_egui, |ui| {
            ui.set_width(380.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("New Note").color(t.text).strong().size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_close_button(ui, &t).clicked() {
                        close = true;
                    }
                });
            });

            ui.add_space(8.0);
            ui.label(RichText::new("Note title or folder path (e.g. 'Roadmap' or 'Docs/Intro'):").color(t.text_dim).size(12.0));

            ui.add_space(4.0);
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.new_note_name)
                    .hint_text("Note Title")
                    .desired_width(ui.available_width()),
            );
            edit.request_focus();

            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                create_note = true;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Color:").color(t.text_dim).size(12.0));
                ui.add_space(4.0);
                if let Some(action) = theme::color_picker_dots(ui, &t, ctx.settings.effective_color_palette(), state.new_note_color.as_deref()) {
                    match action {
                        theme::ColorPickerAction::SetColor(c) => state.new_note_color = c,
                        theme::ColorPickerAction::OpenCustomColor => {
                            state.custom_color_target = Some(CustomColorTarget::NewNoteDialog);
                            state.custom_color_hex = state.new_note_color.clone().unwrap_or_else(|| "#2d9cdb".into());
                        }
                    }
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Emoji:").color(t.text_dim).size(12.0));
                ui.add_space(4.0);
                for em in ["📝", "📄", "💡", "🚀", "⭐", "📌", "🎯", "🔍"] {
                    if ui.button(RichText::new(em)).clicked() {
                        state.new_note_emoji = Some(em.to_string());
                    }
                }
            });

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Cancel").color(t.text_dim)).clicked() {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_create = !state.new_note_name.trim().is_empty();
                    if ui.add_enabled(can_create, egui::Button::new(RichText::new("Create Note").strong().color(Color32::WHITE)).fill(t.accent)).clicked() {
                        create_note = true;
                    }
                });
            });
        });

        if create_note {
            let typed = state.new_note_name.trim();
            let full_name = match &state.new_note_folder {
                Some(f) if !f.is_empty() && !typed.contains('/') => format!("{f}/{typed}"),
                _ => typed.to_string(),
            };

            if let Ok(id) = NoteId::new(&full_name) {
                let mut initial = String::new();
                let has_fm = state.new_note_color.is_some() || state.new_note_emoji.is_some();
                if has_fm {
                    initial.push_str("---\n");
                    if let Some(ref c) = state.new_note_color {
                        initial.push_str(&format!("color: {c}\n"));
                    }
                    if let Some(ref e) = state.new_note_emoji {
                        initial.push_str(&format!("icon: {e}\n"));
                    }
                    initial.push_str("---\n\n");
                }
                initial.push_str(&format!("# {}\n\n", id.name()));

                if let Ok(_) = ctx.vault.create(id.clone(), &initial) {
                    *ctx.index = LinkIndex::build(ctx.vault);
                    ctx.open_note(id);
                    ctx.activate_plugin("core.markdown");
                    ctx.toast("Note created");
                }
            }
            close = true;
        }

        if modal_resp.should_close() || close {
            state.show_new_note = false;
            state.new_note_name.clear();
            state.new_note_folder = None;
            state.new_note_emoji = None;
            state.new_note_color = None;
        }
    }

    // 2. Rename Note Dialog
    if state.show_rename_note {
        let modal = egui::Modal::new(Id::unique("rename_note_modal"))
            .backdrop_color(Color32::from_black_alpha(150))
            .frame(theme::modal_frame(&t));

        let mut close = false;
        let mut do_rename = false;

        let modal_resp = modal.show(ctx_egui, |ui| {
            ui.set_width(380.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Rename / Move Note").color(t.text).strong().size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_close_button(ui, &t).clicked() {
                        close = true;
                    }
                });
            });

            ui.add_space(8.0);
            ui.label(RichText::new("Enter new name or path. All [[wiki-links]] across your vault will be updated.").color(t.text_dim).size(12.0));

            ui.add_space(6.0);
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.rename_new_name)
                    .desired_width(ui.available_width()),
            );
            edit.request_focus();

            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_rename = true;
            }

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("Rename Note").strong().color(Color32::WHITE)).clicked() {
                        do_rename = true;
                    }
                });
            });
        });

        if do_rename {
            if let Some(ref target) = state.rename_target {
                let new_raw = state.rename_new_name.trim();
                if let Ok(new_id) = NoteId::new(new_raw) {
                    if let Ok(_) = ctx.vault.rename(target, new_id.clone()) {
                        ctx.emit(AppEvent::NoteRenamed {
                            old: target.clone(),
                            new: new_id.clone(),
                        });
                        ctx.open_note(new_id);
                        ctx.toast("Note renamed and links updated");
                    }
                }
            }
            close = true;
        }

        if modal_resp.should_close() || close {
            state.show_rename_note = false;
            state.rename_target = None;
            state.rename_new_name.clear();
        }
    }

    // 3. New Folder Dialog
    if state.show_new_folder {
        let modal = egui::Modal::new(Id::unique("new_folder_modal"))
            .backdrop_color(Color32::from_black_alpha(150))
            .frame(theme::modal_frame(&t));

        let mut close = false;
        let mut do_create = false;

        let modal_resp = modal.show(ctx_egui, |ui| {
            ui.set_width(360.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("New Folder").color(t.text).strong().size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::icon_close_button(ui, &t).clicked() {
                        close = true;
                    }
                });
            });

            ui.add_space(8.0);
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.new_folder_name)
                    .hint_text("Folder Name")
                    .desired_width(ui.available_width()),
            );
            edit.request_focus();

            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_create = true;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Color:").color(t.text_dim).size(12.0));
                ui.add_space(4.0);
                if let Some(action) = theme::color_picker_dots(ui, &t, ctx.settings.effective_color_palette(), state.new_folder_color.as_deref()) {
                    match action {
                        theme::ColorPickerAction::SetColor(c) => state.new_folder_color = c,
                        theme::ColorPickerAction::OpenCustomColor => {
                            state.custom_color_target = Some(CustomColorTarget::NewFolderDialog);
                        }
                    }
                }
            });

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_create = !state.new_folder_name.trim().is_empty();
                    if ui.add_enabled(can_create, egui::Button::new(RichText::new("Create Folder").strong().color(Color32::WHITE)).fill(t.accent)).clicked() {
                        do_create = true;
                    }
                });
            });
        });

        if do_create {
            let name = state.new_folder_name.trim();
            let full_path = match &state.new_folder_parent {
                Some(p) if !p.is_empty() => format!("{p}/{name}"),
                _ => name.to_string(),
            };

            if let Ok(_) = ctx.vault.create_folder(&full_path) {
                if let Some(ref c) = state.new_folder_color {
                    ctx.settings.folder_colors.insert(full_path.clone(), c.clone());
                }
                if let Some(ref e) = state.new_folder_emoji {
                    ctx.settings.folder_emojis.insert(full_path.clone(), e.clone());
                }
                let _ = ctx.settings.save_to_file(&std::path::PathBuf::from("settings.json"));
                ctx.toast(format!("Created folder: {full_path}"));
            }
            close = true;
        }

        if modal_resp.should_close() || close {
            state.show_new_folder = false;
            state.new_folder_name.clear();
            state.new_folder_parent = None;
            state.new_folder_emoji = None;
            state.new_folder_color = None;
        }
    }

    // 4. Custom Color Dialog
    if let Some(ref target) = state.custom_color_target.clone() {
        let modal = egui::Modal::new(Id::unique("custom_color_modal"))
            .backdrop_color(Color32::from_black_alpha(150))
            .frame(theme::modal_frame(&t));

        let mut close = false;
        let mut apply_color = false;

        let modal_resp = modal.show(ctx_egui, |ui| {
            ui.set_width(340.0);
            ui.heading(RichText::new("Custom Color").color(t.text).strong().size(16.0));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Hex:").color(t.text_dim).size(12.0));
                ui.add(egui::TextEdit::singleline(&mut state.custom_color_hex).hint_text("#RRGGBB").desired_width(100.0));
                let parsed = theme::parse_hex_color(&state.custom_color_hex).unwrap_or(t.text_dim);
                let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 8.0, parsed);
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Label:").color(t.text_dim).size(12.0));
                ui.add(egui::TextEdit::singleline(&mut state.custom_color_name).hint_text("Optional palette label").desired_width(140.0));
            });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let valid = theme::parse_hex_color(&state.custom_color_hex).is_some();
                    if ui.add_enabled(valid, egui::Button::new(RichText::new("Apply").color(Color32::WHITE)).fill(t.accent)).clicked() {
                        apply_color = true;
                    }
                });
            });
        });

        if apply_color {
            let hex = state.custom_color_hex.trim().to_string();
            match target {
                CustomColorTarget::Note(ref id) => {
                    if let Some(note) = ctx.vault.get(id) {
                        let (mut fm, body) = Frontmatter::parse(&note.content);
                        fm.set("color", &hex);
                        let new_content = if fm.is_empty() {
                            body.trim_start_matches('\n').to_string()
                        } else {
                            format!("{}{}", fm.render(), body.trim_start_matches('\n'))
                        };
                        let _ = ctx.vault.save(id, &new_content);
                        *ctx.index = LinkIndex::build(ctx.vault);
                        ctx.emit(AppEvent::NoteModified(id.clone()));
                    }
                }
                CustomColorTarget::Folder(ref path) => {
                    ctx.settings.folder_colors.insert(path.clone(), hex.clone());
                }
                CustomColorTarget::NewNoteDialog => {
                    state.new_note_color = Some(hex.clone());
                }
                CustomColorTarget::NewFolderDialog => {
                    state.new_folder_color = Some(hex.clone());
                }
            }
            if !state.custom_color_name.trim().is_empty() {
                let label = state.custom_color_name.trim().to_string();
                let name = label.to_lowercase().replace(' ', "-");
                ctx.settings.color_palette.push(ColorDef { name, label, hex });
            }
            let _ = ctx.settings.save_to_file(&std::path::PathBuf::from("settings.json"));
            close = true;
        }

        if modal_resp.should_close() || close {
            state.custom_color_target = None;
            state.custom_color_name.clear();
        }
    }
}
