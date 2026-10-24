//! Events Agenda and Calendar View.

use chrono::Local;
use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};

use crate::state::AppState;
use crate::theme::{
    ACCENT_AMBER, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_PURPLE_LIGHT, BG_CARD, BORDER_COLOR,
    TEXT_FAINT, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("🗓 Calendar & Events").color(TEXT_PRIMARY).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("+ New Event").color(ACCENT_GREEN)).clicked() {
                state.new_event_draft.date = Local::now().date_naive();
                state.show_new_event_dialog = true;
            }
        });
    });

    ui.separator();

    let events = state.event_store.all(&state.vault);

    if events.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("No events scheduled.\nClick '+ New Event' to create one!").color(TEXT_MUTED));
        });
        return;
    }

    ScrollArea::vertical().id_salt("events_agenda_scroll").show(ui, |ui| {
        let today = Local::now().date_naive();

        for event in events {
            let is_today = event.date == today;
            let is_past = event.date < today;

            Frame::NONE
                .fill(BG_CARD)
                .stroke(Stroke::new(1.0, if is_today { ACCENT_PURPLE } else { BORDER_COLOR }))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Date column
                        ui.vertical(|ui| {
                            let date_color = if is_today {
                                ACCENT_PURPLE_LIGHT
                            } else if is_past {
                                TEXT_FAINT
                            } else {
                                TEXT_PRIMARY
                            };
                            ui.label(RichText::new(event.date.format("%a, %b %-d").to_string()).strong().color(date_color));
                            ui.label(RichText::new(event.time_label()).color(ACCENT_AMBER).size(12.0));
                        });

                        ui.separator();

                        // Title and details column
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&event.title).size(15.0).strong().color(TEXT_PRIMARY));
                            if let Some(loc) = &event.location {
                                ui.label(RichText::new(format!("📍 {loc}")).color(TEXT_MUTED).size(12.0));
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Open Note ↗").color(ACCENT_PURPLE_LIGHT)).clicked() {
                                state.open_note(event.id.clone());
                            }
                        });
                    });
                });

            ui.add_space(6.0);
        }
    });
}
