//! Daily Notes timeline and date navigator.

use chrono::Local;
use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};

use crate::state::AppState;
use crate::theme::{
    ACCENT_PURPLE, ACCENT_PURPLE_LIGHT, BG_CARD, BG_HOVER, BORDER_COLOR, TEXT_FAINT, TEXT_MUTED,
    TEXT_PRIMARY,
};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("📅 Daily Notes").color(TEXT_PRIMARY).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("Today").color(ACCENT_PURPLE_LIGHT)).clicked() {
                state.open_daily_today();
            }
        });
    });

    ui.separator();

    // Date switcher buttons: Previous Day, Selected Date, Next Day
    ui.horizontal(|ui| {
        if ui.button("◀ Previous Day").clicked() {
            state.shift_daily_date(-1);
        }

        let date_str = state.selected_daily_date.format("%A, %B %-d, %Y").to_string();
        ui.label(RichText::new(date_str).strong().color(ACCENT_PURPLE_LIGHT));

        if ui.button("Next Day ▶").clicked() {
            state.shift_daily_date(1);
        }
    });

    ui.add_space(8.0);
    ui.label(RichText::new("Existing Daily Notes").color(TEXT_MUTED).size(12.0));

    ScrollArea::vertical().id_salt("daily_list_scroll").show(ui, |ui| {
        let existing = state.daily_config.existing_dates(&state.vault);
        if existing.is_empty() {
            ui.label(RichText::new("No daily notes yet. Click Today above to create your first!").color(TEXT_FAINT));
        } else {
            // Sort in reverse chronological order
            let mut dates: Vec<_> = existing.into_iter().collect();
            dates.sort_by(|a, b| b.cmp(a));

            for date in dates {
                let id = state.daily_config.id_for(date);
                let is_today = date == Local::now().date_naive();
                let is_active = state.active_note.as_ref() == Some(&id);

                let mut frame = Frame::NONE
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.0, if is_active { ACCENT_PURPLE } else { BORDER_COLOR }))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8));

                if is_active {
                    frame = frame.fill(BG_HOVER);
                }

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(date.format("%Y-%m-%d").to_string()).strong().color(TEXT_PRIMARY));
                        ui.label(RichText::new(date.format("%A").to_string()).color(TEXT_MUTED).size(12.0));

                        if is_today {
                            ui.label(RichText::new("Today").color(ACCENT_PURPLE_LIGHT).size(11.0));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("Open ↗").color(ACCENT_PURPLE_LIGHT)).clicked() {
                                state.open_daily_date(date);
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        }
    });
}
