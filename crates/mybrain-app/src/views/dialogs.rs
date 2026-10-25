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

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let can_create = !state.new_note_name.trim().is_empty();
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
            ui.heading(RichText::new("New Calendar Event").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(10.0);

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
                egui::Button::new(RichText::new("Save Event").color(Color32::WHITE).strong())
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

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let can_create = !state.new_folder_name.trim().is_empty();
            if ui.add_enabled(
                can_create,
                egui::Button::new(RichText::new("Create Folder").color(Color32::WHITE).strong())
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
