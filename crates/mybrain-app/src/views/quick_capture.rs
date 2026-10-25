//! Quick capture modal dialog.

use egui::{Color32, CornerRadius, Frame, Id, Margin, RichText, Stroke};
use mybrain_core::NoteId;

use crate::state::AppState;
use crate::theme::{self, Tokens};

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_quick_capture {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut should_capture = false;
    let mut note_to_open: Option<NoteId> = None;

    let recent: Vec<NoteId> = state
        .quick_config
        .recent(&state.vault, 4)
        .into_iter()
        .map(|n| n.id.clone())
        .collect();

    let modal = egui::Modal::new(Id::unique("quick_capture_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(480.0);

        ui.horizontal(|ui| {
            ui.heading(RichText::new("Quick Note Capture").color(t.text).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(2.0);
        ui.label(
            RichText::new("Fleeting thoughts are saved to Quick Notes/ and linked in today's daily note.")
                .color(t.text_dim)
                .size(11.5),
        );

        ui.add_space(10.0);

        // Text area
        Frame::NONE
            .fill(t.field)
            .stroke(Stroke::new(1.0, t.field_border))
            .corner_radius(CornerRadius::same(t.radius_sm as u8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut state.quick_capture_buffer)
                        .hint_text("What's on your mind? (Supports [[wiki links]] and Markdown)")
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .desired_rows(6),
                );
                edit.request_focus();

                if edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command) {
                    should_capture = true;
                }
            });

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Save Thought").color(Color32::WHITE).strong())
                        .fill(t.warning)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                )
                .clicked()
            {
                should_capture = true;
            }

            if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                should_close = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Ctrl+Enter to save").color(t.text_faint).size(10.5));
            });
        });

        if !recent.is_empty() {
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);

            theme::section_label(ui, &t, "Recent Quick Notes");
            ui.add_space(2.0);
            for id in recent {
                let dot = theme::note_kind_color(id.as_str(), &t);
                if theme::list_row(ui, &t, false, Some(dot), id.name(), None).clicked() {
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
    if modal_resp.should_close() || should_close {
        state.show_quick_capture = false;
    }
}
