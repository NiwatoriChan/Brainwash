//! Command palette and Quick Switcher (Ctrl+P / Ctrl+O).

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Window};
use mybrain_core::NoteId;

use crate::state::AppState;
use crate::theme::{
    ACCENT_PURPLE, ACCENT_PURPLE_LIGHT, BG_CARD, TEXT_FAINT, TEXT_PRIMARY,
};

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_command_palette {
        return;
    }

    let mut open = true;
    let mut should_close = false;
    let mut note_to_open: Option<NoteId> = None;
    let mut note_to_create: Option<String> = None;

    let hits: Vec<NoteId> = if state.command_palette_query.is_empty() {
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

    Window::new("Quick Switcher")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(500.0)
        .default_height(350.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -80.0])
        .frame(
            Frame::NONE
                .fill(BG_CARD)
                .stroke(Stroke::new(1.5, ACCENT_PURPLE))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(12)),
        )
        .show(ctx, |ui| {
            let search_edit = ui.add(
                egui::TextEdit::singleline(&mut state.command_palette_query)
                    .hint_text("Search notes by title or content...")
                    .desired_width(f32::INFINITY),
            );

            search_edit.request_focus();

            ui.add_space(8.0);
            ui.separator();

            ScrollArea::vertical().id_salt("cmd_palette_scroll").show(ui, |ui| {
                if hits.is_empty() {
                    ui.label(RichText::new("No matching notes found.").color(TEXT_FAINT));
                    if !state.command_palette_query.trim().is_empty() {
                        let query = state.command_palette_query.trim().to_string();
                        if ui.button(RichText::new(format!("+ Create \"{}\"", query)).color(ACCENT_PURPLE_LIGHT)).clicked() {
                            note_to_create = Some(query);
                            should_close = true;
                        }
                    }
                } else {
                    for id in hits {
                        let title = id.name().to_string();
                        let folder = id.folder().map(|s| s.to_string());

                        let btn_frame = Frame::NONE
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(8, 6));

                        btn_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("📄").size(14.0));
                                if ui.selectable_label(false, RichText::new(&title).strong().color(TEXT_PRIMARY)).clicked() {
                                    note_to_open = Some(id.clone());
                                    should_close = true;
                                }

                                if let Some(f) = folder {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(f).color(TEXT_FAINT).size(10.0));
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
    if let Some(query) = note_to_create {
        state.create_note(&query);
    }
    if !open || should_close {
        state.show_command_palette = false;
    }
}
