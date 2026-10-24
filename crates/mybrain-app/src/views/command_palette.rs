//! Command Palette / Fast Switcher modal (Ctrl+P / Ctrl+O).

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Window};
use mybrain_core::NoteId;

use crate::state::AppState;
use crate::theme::Tokens;

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_command_palette {
        return;
    }

    let t = Tokens::get(ctx);
    let mut open = true;
    let mut should_close = false;
    let mut note_to_open: Option<NoteId> = None;
    let mut note_to_create: Option<String> = None;

    Window::new("Quick Switcher")
        .open(&mut open)
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .default_width(520.0)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
        .frame(
            Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.card_border))
                .corner_radius(CornerRadius::same(t.radius_lg as u8))
                .inner_margin(Margin::same(14)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Find:").color(t.text_dim).strong());
                let text_edit = egui::TextEdit::singleline(&mut state.command_palette_query)
                    .hint_text("Type note name or prefix to search...")
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY);
                let response = ui.add(text_edit);
                response.request_focus();
            });

            ui.add_space(8.0);
            ui.separator();

            let hits: Vec<NoteId> = if state.command_palette_query.trim().is_empty() {
                state.vault.notes().take(12).map(|n| n.id.clone()).collect()
            } else {
                state
                    .vault
                    .search(&state.command_palette_query)
                    .into_iter()
                    .take(15)
                    .map(|h| h.id)
                    .collect()
            };

            ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;

                if hits.is_empty() {
                    ui.label(RichText::new("No matching notes found.").color(t.text_faint));
                    if !state.command_palette_query.trim().is_empty() {
                        let query = state.command_palette_query.trim().to_string();
                        if ui.button(RichText::new(format!("+ Create \"{}\"", query)).color(t.accent_text)).clicked() {
                            note_to_create = Some(query);
                            should_close = true;
                        }
                    }
                } else {
                    for id in hits {
                        let title = id.name().to_string();
                        let folder = id.folder().map(|s| s.to_string());

                        let btn_frame = Frame::NONE
                            .corner_radius(CornerRadius::same(t.radius_sm as u8))
                            .inner_margin(Margin::symmetric(8, 6));

                        btn_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("•").color(t.accent_text));
                                if ui.selectable_label(false, RichText::new(&title).strong().color(t.text)).clicked() {
                                    note_to_open = Some(id.clone());
                                    should_close = true;
                                }

                                if let Some(f) = folder {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(f).color(t.text_faint).size(10.0));
                                    });
                                }
                            });
                        });
                    }
                }
            });

            // Close on Escape
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                should_close = true;
            }
        });

    if let Some(id) = note_to_open {
        state.open_note(id);
    }
    if let Some(target) = note_to_create {
        state.open_or_create(&target);
    }
    if !open || should_close {
        state.show_command_palette = false;
        state.command_palette_query.clear();
    }
}
