//! Sidebar explorer: note search, file list, quick actions, and vault stats.

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::NoteId;

use crate::state::{AppState, MainTab};
use crate::theme::{
    ACCENT_AMBER, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_PURPLE_LIGHT, BG_CARD, BG_HOVER,
    BORDER_COLOR, TEXT_FAINT, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.spacing_mut().item_spacing.y = 8.0;

    // Header brand
    ui.horizontal(|ui| {
        ui.label(RichText::new("🧠").size(20.0));
        ui.heading(RichText::new("MyBrain").color(TEXT_PRIMARY).strong().size(18.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("⚡ Capture").color(ACCENT_AMBER)).on_hover_text("Quick capture thought (Ctrl+Q)").clicked() {
                state.show_quick_capture = true;
            }
        });
    });

    ui.separator();

    // Mode Selector (Notes / Daily / Events)
    ui.horizontal(|ui| {
        let tab = state.main_tab;
        if ui.selectable_label(tab == MainTab::Notes, "📄 Notes").clicked() {
            state.main_tab = MainTab::Notes;
        }
        if ui.selectable_label(tab == MainTab::Daily, "📅 Daily").clicked() {
            state.open_daily_today();
        }
        if ui.selectable_label(tab == MainTab::Events, "🗓 Events").clicked() {
            state.main_tab = MainTab::Events;
        }
    });

    // Quick Action Bar
    ui.horizontal(|ui| {
        if ui.button(RichText::new("+ Note").color(ACCENT_PURPLE_LIGHT)).clicked() {
            state.show_new_note_dialog = true;
        }
        if ui.button(RichText::new("+ Event").color(ACCENT_GREEN)).clicked() {
            state.show_new_event_dialog = true;
        }
        if ui.button(RichText::new("🔍 Find").color(TEXT_MUTED)).on_hover_text("Command palette (Ctrl+P)").clicked() {
            state.show_command_palette = true;
        }
    });

    // Search Box
    Frame::NONE
        .fill(BG_CARD)
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔎").color(TEXT_FAINT));
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Filter notes...")
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY),
                );
                if !state.search_query.is_empty() && ui.small_button("✕").clicked() {
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

    ui.label(RichText::new(format!("Notes ({})", note_ids.len())).color(TEXT_MUTED).size(11.0));

    ScrollArea::vertical().id_salt("sidebar_notes_scroll").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;

        for id in note_ids {
            let is_selected = state.active_note.as_ref() == Some(&id);
            let title = id.name();
            let folder = id.folder();

            let mut frame = Frame::NONE.corner_radius(CornerRadius::same(4)).inner_margin(Margin::symmetric(6, 4));
            if is_selected {
                frame = frame.fill(BG_HOVER).stroke(Stroke::new(1.0, ACCENT_PURPLE));
            }

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let icon = if id.as_str().starts_with("Daily/") {
                        "📅"
                    } else if id.as_str().starts_with("Events/") {
                        "🗓"
                    } else if id.as_str().starts_with("Quick Notes/") {
                        "⚡"
                    } else {
                        "📄"
                    };

                    ui.label(RichText::new(icon).size(13.0));

                    let label_text = if is_selected {
                        RichText::new(title).color(ACCENT_PURPLE_LIGHT).strong()
                    } else {
                        RichText::new(title).color(TEXT_PRIMARY)
                    };

                    let response = ui.selectable_label(is_selected, label_text);
                    if response.clicked() {
                        state.open_note(id.clone());
                    }

                    if let Some(f) = folder {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(f).color(TEXT_FAINT).size(10.0));
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
            ui.label(RichText::new(format!("{total_notes} notes")).color(TEXT_FAINT).size(11.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((msg, _)) = &state.status {
                    ui.label(RichText::new(msg).color(ACCENT_PURPLE_LIGHT).size(11.0));
                }
            });
        });
    });
}
