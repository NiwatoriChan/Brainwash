//! Modals for creating new notes and new calendar events.

use chrono::NaiveDate;
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Window};
use mybrain_core::events::parse_time;

use crate::state::AppState;
use crate::theme::{
    ACCENT_GREEN, ACCENT_PURPLE_LIGHT, BG_CARD, BORDER_COLOR, TEXT_MUTED,
};

pub fn show_new_note(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_new_note_dialog {
        return;
    }

    let mut open = true;
    let mut should_close = false;
    let mut note_to_create: Option<String> = None;

    Window::new("Create New Note")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(380.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -50.0])
        .frame(
            Frame::NONE
                .fill(BG_CARD)
                .stroke(Stroke::new(1.0, BORDER_COLOR))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(14)),
        )
        .show(ctx, |ui| {
            ui.label(RichText::new("Note Name (e.g. 'Projects/Roadmap'):").color(TEXT_MUTED));
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.new_note_name)
                    .desired_width(f32::INFINITY),
            );
            edit.request_focus();

            let submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if (ui.button(RichText::new("Create").color(ACCENT_PURPLE_LIGHT).strong()).clicked() || submitted)
                    && !state.new_note_name.trim().is_empty()
                {
                    note_to_create = Some(state.new_note_name.clone());
                    should_close = true;
                }
                if ui.button("Cancel").clicked() {
                    should_close = true;
                }
            });
        });

    if let Some(name) = note_to_create {
        state.create_note(&name);
    }
    if !open || should_close {
        state.show_new_note_dialog = false;
    }
}

pub fn show_new_event(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_new_event_dialog {
        return;
    }

    let mut open = true;
    let mut should_close = false;
    let mut should_create = false;

    Window::new("🗓 New Calendar Event")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(420.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -50.0])
        .frame(
            Frame::NONE
                .fill(BG_CARD)
                .stroke(Stroke::new(1.0, BORDER_COLOR))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(14)),
        )
        .show(ctx, |ui| {
            ui.label(RichText::new("Event Title:").color(TEXT_MUTED));
            ui.add(egui::TextEdit::singleline(&mut state.new_event_draft.title).desired_width(f32::INFINITY));

            ui.add_space(6.0);

            let mut date_str = state.new_event_draft.date.format("%Y-%m-%d").to_string();
            ui.label(RichText::new("Date (YYYY-MM-DD):").color(TEXT_MUTED));
            if ui.add(egui::TextEdit::singleline(&mut date_str).desired_width(f32::INFINITY)).changed() {
                if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    state.new_event_draft.date = d;
                }
            }

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Start (HH:MM):").color(TEXT_MUTED));
                    let mut start_str = state
                        .new_event_draft
                        .start
                        .map(|t| t.format("%H:%M").to_string())
                        .unwrap_or_default();
                    if ui.add(egui::TextEdit::singleline(&mut start_str).desired_width(120.0)).changed() {
                        state.new_event_draft.start = parse_time(&start_str).ok().flatten();
                    }
                });

                ui.vertical(|ui| {
                    ui.label(RichText::new("End (HH:MM):").color(TEXT_MUTED));
                    let mut end_str = state
                        .new_event_draft
                        .end
                        .map(|t| t.format("%H:%M").to_string())
                        .unwrap_or_default();
                    if ui.add(egui::TextEdit::singleline(&mut end_str).desired_width(120.0)).changed() {
                        state.new_event_draft.end = parse_time(&end_str).ok().flatten();
                    }
                });
            });

            ui.add_space(6.0);

            ui.label(RichText::new("Location:").color(TEXT_MUTED));
            ui.add(egui::TextEdit::singleline(&mut state.new_event_draft.location).desired_width(f32::INFINITY));

            ui.add_space(6.0);

            ui.label(RichText::new("Description (Supports [[wiki links]]):").color(TEXT_MUTED));
            ui.add(egui::TextEdit::multiline(&mut state.new_event_draft.description).desired_rows(3).desired_width(f32::INFINITY));

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button(RichText::new("Save Event").color(ACCENT_GREEN).strong()).clicked() {
                    should_create = true;
                    should_close = true;
                }
                if ui.button("Cancel").clicked() {
                    should_close = true;
                }
            });
        });

    if should_create {
        state.create_event();
    }
    if !open || should_close {
        state.show_new_event_dialog = false;
    }
}
