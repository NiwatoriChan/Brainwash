//! Events Agenda and Calendar View.

use chrono::Local;
use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};

use crate::state::AppState;
use crate::theme::Tokens;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    Frame::NONE
        .fill(t.canvas)
        .inner_margin(Margin::symmetric(28, 20))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Calendar & Events").color(t.text).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("+ New Event").color(t.success)).clicked() {
                        state.new_event_draft.date = Local::now().date_naive();
                        state.show_new_event_dialog = true;
                    }
                });
            });

            ui.separator();

    let events = state.event_store.all(&state.vault);

    if events.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("No events scheduled.\nClick '+ New Event' to create one!").color(t.text_dim));
        });
        return;
    }

    ScrollArea::vertical().id_salt("events_agenda_scroll").show(ui, |ui| {
        let today = Local::now().date_naive();

        for event in events {
            let is_today = event.date == today;
            let is_past = event.date < today;

            Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, if is_today { t.accent_border } else { t.card_border }))
                .corner_radius(CornerRadius::same(t.radius_sm as u8))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Date column
                        ui.vertical(|ui| {
                            let date_color = if is_today {
                                t.accent_text
                            } else if is_past {
                                t.text_faint
                            } else {
                                t.text
                            };
                            ui.label(RichText::new(event.date.format("%a, %b %-d").to_string()).strong().color(date_color));
                            ui.label(RichText::new(event.time_label()).color(t.warning).size(12.0));
                        });

                        ui.separator();

                        // Title and details column
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&event.title).size(15.0).strong().color(t.text));
                            if let Some(loc) = &event.location {
                                ui.label(RichText::new(format!("Location: {loc}")).color(t.text_dim).size(12.0));
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Open Note →").color(t.accent_text)).clicked() {
                                state.open_note(event.id.clone());
                            }
                        });
                    });
                });

            ui.add_space(6.0);
        }
    });
    });
}
