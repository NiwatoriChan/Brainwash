//! Events Agenda and Calendar View.

use chrono::Local;
use egui::{Color32, Frame, Margin, RichText, ScrollArea, Ui};
use mybrain_core::events::EventDraft;

use crate::state::AppState;
use crate::theme::{self, Tokens};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    ScrollArea::vertical()
        .id_salt("events_view_scroll")
        .show(ui, |ui| {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(theme::READABLE_WIDTH);

                // Header Card
                let header_card = theme::card_frame(&t, false)
                    .inner_margin(Margin::symmetric(18, 14));

                let events = state.event_store.all(&state.vault);

                header_card.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("Calendar & Events").color(t.text).strong().size(16.0));
                        ui.label(RichText::new(format!("({} scheduled)", events.len())).color(t.text_dim).size(12.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(
                                egui::Button::new(RichText::new("+ New Event").color(Color32::WHITE).strong())
                                    .fill(t.success)
                                    .stroke(egui::Stroke::NONE)
                                    .corner_radius(egui::CornerRadius::same(t.radius_sm as u8)),
                            ).clicked() {
                                state.new_event_draft = EventDraft::new(state.selected_daily_date);
                                state.show_new_event_dialog = true;
                            }
                        });
                    });
                });

                ui.add_space(16.0);

                if events.is_empty() {
                    Frame::NONE
                        .fill(t.card)
                        .stroke(egui::Stroke::new(1.0, t.card_border))
                        .corner_radius(egui::CornerRadius::same(t.radius as u8))
                        .inner_margin(Margin::same(24))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("No upcoming events scheduled.").color(t.text_dim).size(14.0));
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("Add an ```event object block to any note or click '+ New Event'.")
                                        .color(t.text_faint)
                                        .size(12.0),
                                );
                            });
                        });
                } else {
                    let today = Local::now().date_naive();

                    for event in events {
                        let is_today = event.date == today;
                        let is_past = event.date < today;

                        let card = theme::card_frame(&t, is_today)
                            .inner_margin(Margin::same(12));

                        card.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Date / Time column
                                ui.allocate_ui_with_layout(
                                    egui::vec2(110.0, 36.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        let date_color = if is_today {
                                            t.accent_text
                                        } else if is_past {
                                            t.text_faint
                                        } else {
                                            t.text
                                        };
                                        if ui
                                            .add(
                                                egui::Label::new(
                                                    RichText::new(event.date.format("%a, %b %-d").to_string())
                                                        .strong()
                                                        .size(13.0)
                                                        .color(date_color),
                                                )
                                                .sense(egui::Sense::click()),
                                            )
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .on_hover_text("Open Daily Note")
                                            .clicked()
                                        {
                                            state.open_daily_date(event.date);
                                        }
                                        theme::pill(ui, &event.time_label(), t.warning, t.accent_soft);
                                    },
                                );

                                ui.separator();

                                // Details column
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&event.title).size(14.5).strong().color(t.text));
                                    ui.horizontal(|ui| {
                                        theme::pill(ui, &format!("in [[{}]]", event.id.name()), t.accent_text, t.accent_soft);
                                        if let Some(loc) = &event.location {
                                            theme::icon_pin(ui, 12.0, t.text_dim);
                                            ui.label(RichText::new(loc).color(t.text_dim).size(11.5));
                                        }
                                    });
                                    if let Some(desc) = &event.description {
                                        if !desc.is_empty() {
                                            ui.label(RichText::new(desc).color(t.text_dim).size(11.5).italics());
                                        }
                                    }
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if theme::ghost_button(ui, &t, RichText::new("Open Note →").color(t.accent_text)).clicked() {
                                        state.open_note(event.id.clone());
                                    }
                                });
                            });
                        });

                        ui.add_space(6.0);
                    }
                }
            });
            ui.add_space(40.0);
        });
}
