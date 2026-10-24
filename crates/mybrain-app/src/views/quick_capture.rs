//! Quick capture modal dialog.

use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Window};
use mybrain_core::NoteId;

use crate::state::AppState;
use crate::theme::Tokens;

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_quick_capture {
        return;
    }

    let t = Tokens::get(ctx);
    let mut open = true;
    let mut should_close = false;
    let mut should_capture = false;
    let mut note_to_open: Option<NoteId> = None;

    let recent: Vec<NoteId> = state
        .quick_config
        .recent(&state.vault, 3)
        .into_iter()
        .map(|n| n.id.clone())
        .collect();

    Window::new("Quick Note Capture")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(450.0)
        .default_height(250.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -50.0])
        .frame(
            Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.5, t.warning))
                .corner_radius(CornerRadius::same(t.radius_lg as u8))
                .inner_margin(Margin::same(16)),
        )
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Quickly record a thought, task, or snippet.")
                    .color(t.text_dim)
                    .size(12.0),
            );
            ui.label(
                RichText::new("It will be saved as a separate note and linked in today's Daily Note!")
                    .color(t.text_faint)
                    .size(11.0),
            );

            ui.add_space(8.0);

            let edit_response = ui.add(
                egui::TextEdit::multiline(&mut state.quick_capture_buffer)
                    .hint_text("What's on your mind? (Supports [[wiki links]] and Markdown)")
                    .desired_width(f32::INFINITY)
                    .desired_rows(6),
            );

            if edit_response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command) {
                should_capture = true;
            }

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button(RichText::new("Save Thought").color(t.warning).strong()).clicked() {
                    should_capture = true;
                }

                if ui.button("Cancel").clicked() {
                    should_close = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Ctrl+Enter to save").color(t.text_faint).size(10.0));
                });
            });

            ui.add_space(12.0);
            ui.separator();

            // Recent quick notes
            if !recent.is_empty() {
                ui.label(RichText::new("Recent Quick Notes").color(t.text_dim).size(11.0));
                for id in recent {
                    if ui.button(RichText::new(format!("• {}", id.name())).color(t.accent_text)).clicked() {
                        note_to_open = Some(id);
                        should_close = true;
                    }
                }
            }
        });

    if should_capture {
        state.capture_quick_note();
    }
    if let Some(id) = note_to_open {
        state.open_note(id);
    }
    if !open || should_close {
        state.show_quick_capture = false;
    }
}
