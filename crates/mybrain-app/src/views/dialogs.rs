//! Modals for creating new notes and new calendar events.

use chrono::NaiveDate;
use egui::{Color32, CornerRadius, Frame, Id, Margin, RichText, Stroke};
use mybrain_core::events::parse_time;

use crate::state::AppState;
use crate::theme::{self, Tokens};

fn field_frame(t: &Tokens) -> Frame {
    Frame::NONE
        .fill(t.field)
        .stroke(Stroke::new(1.0, t.field_border))
        .corner_radius(CornerRadius::same(t.radius_sm as u8))
        .inner_margin(Margin::symmetric(10, 6))
}

pub fn show_new_note(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_new_note_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut note_to_create: Option<String> = None;

    let modal = egui::Modal::new(Id::unique("new_note_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(380.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("New Note").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(8.0);
        ui.label(RichText::new("Note title or folder path (e.g. 'Projects/Roadmap'):").color(t.text_dim).size(12.0));

        ui.add_space(4.0);
        let mut submitted = false;
        field_frame(&t).show(ui, |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.new_note_name)
                    .hint_text("My Great Idea")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
            edit.request_focus();
            submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        });

        let typed_name = state.new_note_name.trim();
        let target_folder = match &state.new_note_folder {
            Some(f) if !f.is_empty() && !typed_name.contains('/') => Some(f.clone()),
            _ => {
                if let Some(slash_idx) = typed_name.rfind('/') {
                    Some(typed_name[..slash_idx].to_string())
                } else {
                    None
                }
            }
        };

        let is_reserved = target_folder
            .as_ref()
            .map(|f| state.is_reserved_special_folder(f))
            .unwrap_or(false);

        if is_reserved {
            ui.add_space(4.0);
            ui.label(
                RichText::new("⚠️ Normal notes cannot be added in Daily or Quick Notes folders.")
                    .color(t.warning)
                    .size(11.5),
            );
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Note Emoji / Icon:").color(t.text_dim).size(12.0));
        ui.horizontal_wrapped(|ui| {
            let current_emoji = state.new_note_emoji.as_deref().unwrap_or("📝");
            ui.label(RichText::new(current_emoji).color(t.accent).size(16.0));
            ui.add_space(4.0);
            for em in ["📝", "📄", "💡", "🚀", "⭐", "📌", "🎯", "🔍", "🏷️", "💼", "📚", "🎨"] {
                if ui.button(em).clicked() {
                    state.new_note_emoji = Some(em.to_string());
                }
            }
            if ui.button("More… 🔍").clicked() {
                state.emoji_picker_target = Some(crate::state::EmojiPickerTarget::NewNoteDialog);
                state.emoji_picker_search.clear();
            }
            if state.new_note_emoji.is_some() && ui.button("× Reset").clicked() {
                state.new_note_emoji = None;
            }
        });

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let can_create = !is_reserved && !typed_name.is_empty();
            if ui.add_enabled(
                can_create,
                egui::Button::new(RichText::new("Create Note").color(Color32::WHITE).strong())
                    .fill(t.accent)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
            ).clicked() || (submitted && can_create) {
                note_to_create = Some(state.new_note_name.clone());
                should_close = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }
        });
    });

    if let Some(name) = note_to_create {
        let clean_name = match &state.new_note_folder {
            Some(f) if !f.is_empty() && !name.contains('/') => format!("{f}/{name}"),
            _ => name,
        };
        state.create_note(&clean_name);
        state.new_note_folder = None;
    }
    if modal_resp.should_close() || should_close {
        state.show_new_note_dialog = false;
        state.new_note_folder = None;
    }
}

pub fn show_rename_note(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_rename_note_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut rename_action: Option<(mybrain_core::NoteId, String)> = None;

    let modal = egui::Modal::new(Id::unique("rename_note_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(380.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("Rename or Move Note").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(6.0);
        if let Some(target) = &state.rename_note_target {
            ui.label(
                RichText::new(format!("Current: {}", target.as_str()))
                    .color(t.accent_text)
                    .size(11.5),
            );
        }

        ui.add_space(6.0);
        ui.label(
            RichText::new("New title or folder path (all [[wiki links]] will update automatically):")
                .color(t.text_dim)
                .size(12.0),
        );

        ui.add_space(4.0);
        let mut submitted = false;
        field_frame(&t).show(ui, |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.rename_note_new_name)
                    .hint_text("Projects/Roadmap")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
            edit.request_focus();
            submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        });

        let typed_name = state.rename_note_new_name.trim();
        let target_folder = typed_name
            .rfind('/')
            .map(|slash_idx| &typed_name[..slash_idx]);
        let is_reserved = target_folder
            .map(|f| state.is_reserved_special_folder(f))
            .unwrap_or(false);

        if is_reserved {
            ui.add_space(4.0);
            ui.label(
                RichText::new("⚠️ Cannot move normal notes into Daily or Quick Notes folders.")
                    .color(t.warning)
                    .size(11.5),
            );
        }

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let can_rename = !is_reserved && !typed_name.is_empty() && state.rename_note_target.is_some();
            if ui
                .add_enabled(
                    can_rename,
                    egui::Button::new(RichText::new("Rename Note").color(Color32::WHITE).strong())
                        .fill(t.accent)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                )
                .clicked()
                || (submitted && can_rename)
            {
                if let Some(from) = state.rename_note_target.clone() {
                    rename_action = Some((from, state.rename_note_new_name.clone()));
                }
                should_close = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }
        });
    });

    if let Some((from, new_name)) = rename_action {
        state.rename_note(&from, &new_name);
    }
    if modal_resp.should_close() || should_close {
        state.show_rename_note_dialog = false;
        state.rename_note_target = None;
    }
}

pub fn show_new_event(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_new_event_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut should_create = false;

    let modal = egui::Modal::new(Id::unique("new_event_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(440.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("Insert Event Object").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(4.0);

        let target_desc = if let Some(active) = &state.active_note {
            format!("Inserts into active note: [[{}]]", active.name())
        } else {
            format!("Inserts into daily note: [[Daily/{}]]", state.new_event_draft.date.format("%Y-%m-%d"))
        };
        theme::pill(ui, &target_desc, t.accent_text, t.accent_soft);

        ui.add_space(8.0);

        ui.label(RichText::new("Title").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.new_event_draft.title)
                    .hint_text("Team Retrospective")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
        });

        ui.add_space(6.0);

        let mut date_str = state.new_event_draft.date.format("%Y-%m-%d").to_string();
        ui.label(RichText::new("Date (YYYY-MM-DD)").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            if ui.add(
                egui::TextEdit::singleline(&mut date_str)
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            ).changed() {
                if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    state.new_event_draft.date = d;
                }
            }
        });

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Start (HH:MM)").color(t.text_dim).size(12.0));
                let mut start_str = state
                    .new_event_draft
                    .start
                    .map(|t| t.format("%H:%M").to_string())
                    .unwrap_or_default();
                field_frame(&t).show(ui, |ui| {
                    if ui.add(
                        egui::TextEdit::singleline(&mut start_str)
                            .hint_text("09:00")
                            .frame(Frame::NONE)
                            .desired_width(120.0),
                    ).changed() {
                        state.new_event_draft.start = parse_time(&start_str).ok().flatten();
                    }
                });
            });

            ui.add_space(10.0);

            ui.vertical(|ui| {
                ui.label(RichText::new("End (HH:MM)").color(t.text_dim).size(12.0));
                let mut end_str = state
                    .new_event_draft
                    .end
                    .map(|t| t.format("%H:%M").to_string())
                    .unwrap_or_default();
                field_frame(&t).show(ui, |ui| {
                    if ui.add(
                        egui::TextEdit::singleline(&mut end_str)
                            .hint_text("10:00")
                            .frame(Frame::NONE)
                            .desired_width(120.0),
                    ).changed() {
                        state.new_event_draft.end = parse_time(&end_str).ok().flatten();
                    }
                });
            });
        });

        ui.add_space(6.0);

        ui.label(RichText::new("Location").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.new_event_draft.location)
                    .hint_text("Room 402 or Zoom link")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
        });

        ui.add_space(6.0);

        ui.label(RichText::new("Description (Supports [[wiki links]])").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.new_event_draft.description)
                    .hint_text("Details, agenda notes, and references")
                    .frame(Frame::NONE)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
        });

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let can_save = !state.new_event_draft.title.trim().is_empty();
            if ui.add_enabled(
                can_save,
                egui::Button::new(RichText::new("Insert Event Object").color(Color32::WHITE).strong())
                    .fill(t.success)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
            ).clicked() {
                should_create = true;
                should_close = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }
        });
    });

    if should_create {
        let prev_folder = state.event_store.folder.clone();
        if let Some(f) = &state.new_event_folder {
            state.event_store.folder = f.clone();
        }
        state.create_event();
        state.event_store.folder = prev_folder;
        state.new_event_folder = None;
    }
    if modal_resp.should_close() || should_close {
        state.show_new_event_dialog = false;
        state.new_event_folder = None;
    }
}

pub fn show_new_folder(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_new_folder_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut should_create = false;

    let modal = egui::Modal::new(Id::unique("new_folder_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(360.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("New Folder").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        if let Some(p) = &state.new_folder_parent {
            ui.add_space(2.0);
            ui.label(RichText::new(format!("Parent: {p}")).color(t.accent_text).size(11.5));
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Folder Name:").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.new_folder_name)
                    .hint_text("Documents")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
            edit.request_focus();
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                should_create = true;
                should_close = true;
            }
        });

        let typed_folder = state.new_folder_name.trim();
        let is_reserved_parent = state
            .new_folder_parent
            .as_deref()
            .map(|p| state.is_reserved_special_folder(p))
            .unwrap_or(false);
        let is_reserved_name = !typed_folder.is_empty() && state.is_reserved_special_folder(typed_folder);
        let is_blocked = is_reserved_parent || is_reserved_name;

        if is_blocked {
            ui.add_space(4.0);
            ui.label(
                RichText::new("⚠️ Cannot create folder inside or with reserved name (Daily, Quick Notes).")
                    .color(t.warning)
                    .size(11.5),
            );
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Folder Emoji / Icon:").color(t.text_dim).size(12.0));
        ui.horizontal_wrapped(|ui| {
            let current_emoji = state.new_folder_emoji.as_deref().unwrap_or("📁");
            ui.label(RichText::new(current_emoji).color(t.accent).size(16.0));
            ui.add_space(4.0);
            for em in ["📁", "📂", "🚀", "💡", "💼", "📚", "🎨", "🛠️", "🎯", "🏷️", "📦", "🌟"] {
                if ui.button(em).clicked() {
                    state.new_folder_emoji = Some(em.to_string());
                }
            }
            if ui.button("More… 🔍").clicked() {
                state.emoji_picker_target = Some(crate::state::EmojiPickerTarget::NewFolderDialog);
                state.emoji_picker_search.clear();
            }
            if state.new_folder_emoji.is_some() && ui.button("× Reset").clicked() {
                state.new_folder_emoji = None;
            }
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let can_create = !is_blocked && !typed_folder.is_empty();
            if ui.add_enabled(
                can_create,
                egui::Button::new(RichText::new("Create Folder").color(Color32::WHITE).strong())
                    .fill(t.accent)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
            ).clicked() || (should_create && can_create) {
                should_create = true;
                should_close = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }
        });
    });

    if should_create {
        let parent = state.new_folder_parent.clone();
        let name = state.new_folder_name.clone();
        let _ = state.create_folder(parent.as_deref(), &name);
        state.new_folder_name.clear();
        state.new_folder_parent = None;
    }

    if modal_resp.should_close() || should_close {
        state.show_new_folder_dialog = false;
        state.new_folder_name.clear();
        state.new_folder_parent = None;
    }
}

pub fn show_new_canvas(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_new_canvas_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut should_create = false;

    let modal = egui::Modal::new(Id::unique("new_canvas_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(360.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("New Visual Canvas").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        if let Some(f) = &state.new_canvas_folder {
            ui.add_space(2.0);
            ui.label(RichText::new(format!("Folder: {f}")).color(t.accent_text).size(11.5));
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Canvas Name:").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.new_canvas_name)
                    .hint_text("Architecture Diagram")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
            edit.request_focus();
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                should_create = true;
                should_close = true;
            }
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let can_create = !state.new_canvas_name.trim().is_empty();
            if ui.add_enabled(
                can_create,
                egui::Button::new(RichText::new("Create Canvas").color(Color32::WHITE).strong())
                    .fill(t.accent)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
            ).clicked() {
                should_create = true;
                should_close = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }
        });
    });

    if should_create {
        let folder = state.new_canvas_folder.clone();
        let name = state.new_canvas_name.clone();
        let _ = state.create_canvas(folder.as_deref(), &name);
        state.new_canvas_name.clear();
        state.new_canvas_folder = None;
    }

    if modal_resp.should_close() || should_close {
        state.show_new_canvas_dialog = false;
        state.new_canvas_name.clear();
        state.new_canvas_folder = None;
    }
}

/// Modal dialog for inserting a custom Markdown table.
pub fn show_insert_table(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_insert_table_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut should_insert = false;

    let modal = egui::Modal::new(Id::unique("insert_table_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(380.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("Insert Markdown Table").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Columns:").color(t.text_dim).size(13.0));
            ui.add(egui::DragValue::new(&mut state.insert_table_cols).range(1..=10).speed(0.1));
            ui.add_space(20.0);
            ui.label(RichText::new("Rows:").color(t.text_dim).size(13.0));
            ui.add(egui::DragValue::new(&mut state.insert_table_rows).range(1..=30).speed(0.1));
        });

        ui.add_space(10.0);

        Frame::NONE
            .fill(t.field)
            .stroke(Stroke::new(1.0, t.field_border))
            .corner_radius(CornerRadius::same(t.radius_sm as u8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "Generates a {} × {} table ({} headers + {} data rows)",
                        state.insert_table_cols,
                        state.insert_table_rows,
                        state.insert_table_cols,
                        state.insert_table_rows
                    ))
                    .color(t.text_dim)
                    .size(12.0),
                );
            });

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            if ui.add(
                egui::Button::new(RichText::new("Insert Table").color(Color32::WHITE).strong())
                    .fill(t.accent)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
            ).clicked() {
                should_insert = true;
                should_close = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }
        });
    });

    if should_insert {
        state.insert_table(state.insert_table_cols, state.insert_table_rows);
    }

    if modal_resp.should_close() || should_close {
        state.show_insert_table_dialog = false;
    }
}

/// Modal dialog for inserting a Markdown image.
pub fn show_insert_image(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_insert_image_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut should_insert = false;

    let modal = egui::Modal::new(Id::unique("insert_image_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(420.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("Insert Image").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(10.0);

        ui.label(RichText::new("Alt Text / Caption").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.insert_image_alt)
                    .hint_text("Diagram or Screenshot")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
        });

        ui.add_space(8.0);

        ui.label(RichText::new("Image Path or URL").color(t.text_dim).size(12.0));
        field_frame(&t).show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.insert_image_url)
                    .hint_text("images/architecture.png or https://example.com/photo.png")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new("Tip: Inserts standard Markdown ![alt](url), rendered as an image card in live preview.")
                .color(t.text_faint)
                .size(11.0)
                .italics(),
        );

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let can_insert = !state.insert_image_url.trim().is_empty();
            if ui.add_enabled(
                can_insert,
                egui::Button::new(RichText::new("Insert Image").color(Color32::WHITE).strong())
                    .fill(t.accent)
                    .stroke(Stroke::NONE)
                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
            ).clicked() {
                should_insert = true;
                should_close = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }
        });
    });

    if should_insert {
        let alt = state.insert_image_alt.trim().to_string();
        let url = state.insert_image_url.trim().to_string();
        state.insert_image(&alt, &url);
        state.insert_image_alt.clear();
        state.insert_image_url.clear();
    }

    if modal_resp.should_close() || should_close {
        state.show_insert_image_dialog = false;
    }
}
