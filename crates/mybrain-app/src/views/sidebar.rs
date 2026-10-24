//! Sidebar explorer: note search, file list, quick actions, and vault stats.

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::NoteId;

use crate::state::{AppState, MainTab};
use crate::theme::Tokens;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());
    ui.spacing_mut().item_spacing.y = 8.0;

    // Header brand
    ui.horizontal(|ui| {
        ui.heading(RichText::new("MyBrain").color(t.text).strong().size(18.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("+ Capture").color(t.warning)).on_hover_text("Quick capture thought (Ctrl+Q)").clicked() {
                state.show_quick_capture = true;
            }
        });
    });

    ui.separator();

    // Mode Selector (Notes / Daily / Events)
    ui.horizontal(|ui| {
        let tab = state.main_tab;
        if ui.selectable_label(tab == MainTab::Notes, "Notes").clicked() {
            state.main_tab = MainTab::Notes;
        }
        if ui.selectable_label(tab == MainTab::Daily, "Daily").clicked() {
            state.open_daily_today();
        }
        if ui.selectable_label(tab == MainTab::Events, "Events").clicked() {
            state.main_tab = MainTab::Events;
        }
    });

    // Quick Action Bar
    ui.horizontal(|ui| {
        if ui.button(RichText::new("+ Note").color(t.accent_text)).clicked() {
            state.show_new_note_dialog = true;
        }
        if ui.button(RichText::new("+ Event").color(t.success)).clicked() {
            state.show_new_event_dialog = true;
        }
        if ui.button(RichText::new("Find").color(t.text_dim)).on_hover_text("Command palette (Ctrl+P)").clicked() {
            state.show_command_palette = true;
        }
    });

    // Search Box
    Frame::NONE
        .fill(t.field)
        .corner_radius(CornerRadius::same(t.radius_sm as u8))
        .stroke(Stroke::new(1.0, t.field_border))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("/").color(t.text_faint).strong());
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Filter notes...")
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY),
                );
                if !state.search_query.is_empty() && ui.small_button("×").clicked() {
                    state.search_query.clear();
                }
            });
        });

    ui.separator();

    // Note List
    let note_ids: Vec<NoteId> = if state.search_query.is_empty() {
        state.vault.notes().map(|n| n.id.clone()).collect()
    } else {
        state
            .vault
            .search(&state.search_query)
            .into_iter()
            .map(|hit| hit.id)
            .collect()
    };

    ui.label(RichText::new(format!("Notes ({})", note_ids.len())).color(t.text_dim).size(11.0));

    ScrollArea::vertical().id_salt("sidebar_notes_scroll").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;

        for id in note_ids {
            let is_selected = state.active_note.as_ref() == Some(&id);
            let title = id.name();
            let folder = id.folder();

            let mut frame = Frame::NONE.corner_radius(CornerRadius::same(t.radius_sm as u8)).inner_margin(Margin::symmetric(6, 4));
            if is_selected {
                frame = frame.fill(t.hover).stroke(Stroke::new(1.0, t.accent_border));
            }

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (badge, badge_color) = if id.as_str().starts_with("Daily/") {
                        ("[D]", t.accent_text)
                    } else if id.as_str().starts_with("Events/") {
                        ("[E]", t.warning)
                    } else if id.as_str().starts_with("Quick Notes/") {
                        ("[Q]", t.success)
                    } else {
                        ("•", t.text_dim)
                    };

                    ui.label(RichText::new(badge).color(badge_color).size(11.0).monospace());

                    let label_text = if is_selected {
                        RichText::new(title).color(t.accent_text).strong()
                    } else {
                        RichText::new(title).color(t.text)
                    };

                    let response = ui.selectable_label(is_selected, label_text);
                    if response.clicked() {
                        state.open_note(id.clone());
                    }

                    if let Some(f) = folder {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(f).color(t.text_faint).size(10.0));
                        });
                    }
                });
            });
        }
    });

    // Bottom Stats
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.separator();
        ui.horizontal(|ui| {
            let total_notes = state.vault.len();
            ui.label(RichText::new(format!("{total_notes} notes")).color(t.text_faint).size(11.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((msg, _)) = &state.status {
                    ui.label(RichText::new(msg).color(t.accent_text).size(11.0));
                }
            });
        });
    });
}
