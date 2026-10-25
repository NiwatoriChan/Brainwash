//! Daily Notes calendar and timeline navigator.

use std::collections::BTreeMap;

use chrono::{Datelike, Local, NaiveDate};
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, Frame, Margin, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui,
};
use mybrain_core::events::{Event, EventDraft};
use mybrain_core::NoteId;

use crate::state::{AppState, DailyViewMode};
use crate::theme::{self, Tokens};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    let mut selected_date_to_set: Option<NaiveDate> = None;
    let mut date_to_open: Option<NaiveDate> = None;
    let mut event_to_open: Option<NoteId> = None;
    let mut open_event_dialog = false;
    let mut sync_events_date: Option<NaiveDate> = None;

    ScrollArea::vertical()
        .id_salt("daily_view_scroll")
        .show(ui, |ui| {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(theme::READABLE_WIDTH + 100.0);

                // Top View Switcher & Action Bar
                ui.horizontal(|ui| {
                    if let Some(mode) = theme::segmented(
                        ui,
                        &t,
                        state.daily_view_mode,
                        &[
                            (DailyViewMode::MonthGrid, "Month Calendar"),
                            (DailyViewMode::Agenda, "Events Agenda"),
                            (DailyViewMode::Timeline, "Timeline List"),
                        ],
                    ) {
                        state.daily_view_mode = mode;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::ghost_button(
                            ui,
                            &t,
                            RichText::new("Today").color(t.accent_text).strong(),
                        )
                        .clicked()
                        {
                            let today = Local::now().date_naive();
                            state.selected_daily_date = today;
                            state.calendar_view_date = today;
                        }

                        if theme::ghost_button(
                            ui,
                            &t,
                            RichText::new("+ Event").color(t.success).strong(),
                        )
                        .on_hover_text("Schedule an event for the selected day")
                        .clicked()
                        {
                            state.new_event_draft =
                                EventDraft::new(state.selected_daily_date);
                            state.show_new_event_dialog = true;
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Open Daily Note")
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(t.accent)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                            )
                            .clicked()
                        {
                            date_to_open = Some(state.selected_daily_date);
                        }
                    });
                });

                ui.add_space(14.0);

                match state.daily_view_mode {
                    DailyViewMode::MonthGrid => {
                        render_month_calendar(
                            ui,
                            state,
                            &t,
                            &mut selected_date_to_set,
                            &mut date_to_open,
                            &mut event_to_open,
                            &mut open_event_dialog,
                            &mut sync_events_date,
                        );
                    }
                    DailyViewMode::Agenda => {
                        crate::views::events_view::show(ui, state);
                    }
                    DailyViewMode::Timeline => {
                        render_timeline_list(ui, state, &t, &mut date_to_open);
                    }
                }
            });
            ui.add_space(40.0);
        });

    if let Some(d) = selected_date_to_set {
        state.selected_daily_date = d;
    }
    if let Some(d) = date_to_open {
        state.open_daily_date(d);
    }
    if let Some(id) = event_to_open {
        state.open_note(id);
    }
    if open_event_dialog {
        state.new_event_draft = EventDraft::new(state.selected_daily_date);
        state.show_new_event_dialog = true;
    }
    if let Some(d) = sync_events_date {
        state.sync_events_to_daily_note(d);
    }
}

/// Render the interactive 7-column monthly calendar grid.
fn render_month_calendar(
    ui: &mut Ui,
    state: &mut AppState,
    t: &Tokens,
    selected_date_to_set: &mut Option<NaiveDate>,
    date_to_open: &mut Option<NaiveDate>,
    event_to_open: &mut Option<NoteId>,
    open_event_dialog: &mut bool,
    sync_events_date: &mut Option<NaiveDate>,
) {
    let year = state.calendar_view_date.year();
    let month = state.calendar_view_date.month();
    let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(state.calendar_view_date);
    let lead_padding = first_of_month.weekday().num_days_from_monday();

    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    let days_in_month = (next_month - first_of_month).num_days() as u32;

    // Month Navigation Header Card
    let nav_card = theme::card_frame(t, false).inner_margin(Margin::symmetric(16, 10));
    nav_card.show(ui, |ui| {
        ui.horizontal(|ui| {
            if theme::ghost_button(ui, t, RichText::new("← Prev Month").color(t.text_dim)).clicked() {
                state.shift_calendar_month(-1);
            }

            ui.add_space(8.0);

            let month_title = state.calendar_view_date.format("%B %Y").to_string();
            ui.label(RichText::new(month_title).strong().size(16.0).color(t.text));

            if state.calendar_view_date.year() == Local::now().year()
                && state.calendar_view_date.month() == Local::now().month()
            {
                ui.add_space(4.0);
                theme::pill(ui, "Current Month", t.accent_text, t.accent_soft);
            }

            ui.add_space(8.0);

            if theme::ghost_button(ui, t, RichText::new("Next Month →").color(t.text_dim)).clicked() {
                state.shift_calendar_month(1);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let sel_str = format!("Selected: {}", state.selected_daily_date.format("%a, %b %-d"));
                ui.label(RichText::new(sel_str).size(12.5).color(t.accent_text));
            });
        });
    });

    ui.add_space(10.0);

    // Pre-query daily notes and events for this month
    let existing_daily = state.daily_config.existing_dates(&state.vault);
    let month_events = state
        .event_store
        .between(&state.vault, first_of_month, next_month - chrono::Duration::days(1));

    let mut events_by_day: BTreeMap<u32, Vec<Event>> = BTreeMap::new();
    for ev in month_events {
        events_by_day.entry(ev.date.day()).or_default().push(ev);
    }

    // 7-Column Day Header
    let available_w = ui.available_width().min(theme::READABLE_WIDTH + 100.0);
    let col_gap = 6.0;
    let col_w = ((available_w - col_gap * 6.0) / 7.0).floor();
    let row_h = 76.0;

    let day_names = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = col_gap;
        for name in day_names {
            let (rect, _) = ui.allocate_exact_size(vec2(col_w, 20.0), Sense::hover());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                name,
                egui::FontId::proportional(11.0),
                t.text_faint,
            );
        }
    });

    ui.add_space(4.0);

    // Calendar Grid Rows
    let total_slots = lead_padding + days_in_month;
    let total_rows = (total_slots + 6) / 7;

    for row in 0..total_rows {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = col_gap;

            for col in 0..7 {
                let slot = row * 7 + col;
                if slot < lead_padding || slot >= total_slots {
                    // Empty padding cell
                    let (rect, _) = ui.allocate_exact_size(vec2(col_w, row_h), Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(t.radius_sm as u8),
                        Color32::TRANSPARENT,
                    );
                } else {
                    let day_num = (slot - lead_padding + 1) as u32;
                    let cell_date = NaiveDate::from_ymd_opt(year, month, day_num).unwrap();
                    let is_today = cell_date == Local::now().date_naive();
                    let is_selected = cell_date == state.selected_daily_date;
                    let has_daily = existing_daily.contains(&cell_date);
                    let day_events = events_by_day.get(&day_num);

                    let (rect, resp) =
                        ui.allocate_exact_size(vec2(col_w, row_h), Sense::click());

                    if resp.clicked() {
                        *selected_date_to_set = Some(cell_date);
                    }
                    if resp.double_clicked() {
                        *date_to_open = Some(cell_date);
                    }

                    let radius = CornerRadius::same(t.radius_sm as u8);
                    let bg = if is_selected {
                        t.card
                    } else if resp.hovered() {
                        t.hover
                    } else {
                        t.card
                    };

                    let stroke = if is_selected {
                        Stroke::new(1.8, t.accent)
                    } else if is_today {
                        Stroke::new(1.5, t.accent_border)
                    } else {
                        Stroke::new(1.0, t.card_border)
                    };

                    ui.painter().rect(rect, radius, bg, stroke, egui::StrokeKind::Inside);

                    // Top row of cell: Day number + indicators
                    let top_left = rect.left_top() + vec2(7.0, 6.0);
                    let day_color = if is_today {
                        t.accent_text
                    } else if is_selected {
                        t.text
                    } else {
                        t.text_dim
                    };

                    if is_today {
                        let pill_rect = Rect::from_min_size(
                            top_left - vec2(2.0, 1.0),
                            vec2(22.0, 18.0),
                        );
                        ui.painter().rect_filled(
                            pill_rect,
                            CornerRadius::same(4),
                            t.accent_soft,
                        );
                    }

                    ui.painter().text(
                        top_left + vec2(9.0, 8.0),
                        egui::Align2::CENTER_CENTER,
                        day_num.to_string(),
                        egui::FontId::proportional(12.5),
                        day_color,
                    );

                    // Note dot indicator (violet)
                    if has_daily {
                        let dot_pos = rect.right_top() + vec2(-10.0, 12.0);
                        ui.painter().circle_filled(dot_pos, 3.5, t.accent);
                    }

                    // Event lines in cell (amber)
                    if let Some(events) = day_events {
                        let mut y = rect.top() + 26.0;
                        for ev in events.iter().take(2) {
                            if y + 14.0 > rect.bottom() - 4.0 {
                                break;
                            }
                            ui.painter().circle_filled(
                                pos2(rect.left() + 9.0, y + 6.0),
                                2.5,
                                t.warning,
                            );

                            let max_text_w = col_w - 22.0;
                            let galley = theme::truncated_galley(
                                ui,
                                &ev.title,
                                egui::FontId::proportional(10.0),
                                t.text_dim,
                                max_text_w,
                            );
                            ui.painter().galley(pos2(rect.left() + 16.0, y), galley, t.text_dim);
                            y += 15.0;
                        }

                        if events.len() > 2 {
                            let more_text = format!("+{} more", events.len() - 2);
                            ui.painter().text(
                                pos2(rect.left() + 9.0, rect.bottom() - 10.0),
                                egui::Align2::LEFT_CENTER,
                                more_text,
                                egui::FontId::proportional(9.5),
                                t.text_faint,
                            );
                        }
                    }

                    resp.on_hover_cursor(CursorIcon::PointingHand);
                }
            }
        });
        ui.add_space(col_gap);
    }

    ui.add_space(16.0);

    // Selected Day Detail Card
    render_selected_day_panel(ui, state, t, date_to_open, event_to_open, open_event_dialog, sync_events_date);
}

/// Render the detail panel for the currently selected day.
fn render_selected_day_panel(
    ui: &mut Ui,
    state: &AppState,
    t: &Tokens,
    date_to_open: &mut Option<NaiveDate>,
    event_to_open: &mut Option<NoteId>,
    open_event_dialog: &mut bool,
    sync_events_date: &mut Option<NaiveDate>,
) {
    let sel_date = state.selected_daily_date;
    let is_today = sel_date == Local::now().date_naive();
    let has_daily = state.vault.get(&state.daily_config.id_for(sel_date)).is_some();
    let day_events = state.event_store.on(&state.vault, sel_date);

    let panel = theme::card_frame(t, false).inner_margin(Margin::same(18));
    panel.show(ui, |ui| {
        ui.horizontal(|ui| {
            let title_str = sel_date.format("%A, %B %-d, %Y").to_string();
            ui.heading(RichText::new(title_str).size(16.0).strong().color(t.text));

            if is_today {
                ui.add_space(4.0);
                theme::pill(ui, "Today", t.accent_text, t.accent_soft);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(if has_daily {
                                "Open Daily Note →"
                            } else {
                                "Create Daily Note +"
                            })
                            .strong()
                            .color(Color32::WHITE),
                        )
                        .fill(t.accent)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    )
                    .clicked()
                {
                    *date_to_open = Some(sel_date);
                }

                if theme::ghost_button(
                    ui,
                    t,
                    RichText::new("+ Add Event").color(t.success).strong(),
                )
                .clicked()
                {
                    *open_event_dialog = true;
                }

                if !day_events.is_empty() {
                    if theme::ghost_button(
                        ui,
                        t,
                        RichText::new("Sync to Note").color(t.accent_text).strong(),
                    )
                    .on_hover_text("Synchronize event links into the daily note body")
                    .clicked()
                    {
                        *sync_events_date = Some(sel_date);
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Daily note status line
        ui.horizontal(|ui| {
            if has_daily {
                ui.painter().circle_filled(
                    ui.cursor().min + vec2(4.0, 7.0),
                    3.5,
                    t.success,
                );
                ui.add_space(10.0);
                ui.label(RichText::new("Daily note exists for this date.").color(t.text_dim).size(12.5));
            } else {
                ui.painter().circle_filled(
                    ui.cursor().min + vec2(4.0, 7.0),
                    3.5,
                    t.text_faint,
                );
                ui.add_space(10.0);
                ui.label(RichText::new("No daily note written yet. Click above to create.").color(t.text_faint).size(12.5));
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);

        // Events list for this day
        ui.label(RichText::new("Scheduled Events").strong().size(13.5).color(t.text));
        ui.add_space(4.0);

        if day_events.is_empty() {
            ui.label(RichText::new("No events scheduled for this day.").color(t.text_faint).size(12.5));
        } else {
            for ev in day_events {
                let ev_card = Frame::NONE
                    .fill(t.field)
                    .stroke(Stroke::new(1.0, t.field_border))
                    .corner_radius(CornerRadius::same(t.radius_sm as u8))
                    .inner_margin(Margin::symmetric(12, 8));

                ev_card.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.painter().circle_filled(
                            ui.cursor().min + vec2(4.0, 8.0),
                            3.5,
                            t.warning,
                        );
                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&ev.title).strong().size(13.0).color(t.text));
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(ev.time_label()).size(11.5).color(t.text_dim));
                                if let Some(loc) = &ev.location {
                                    ui.label(RichText::new(format!("• {loc}")).size(11.5).color(t.text_faint));
                                }
                            });
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if theme::ghost_button(ui, t, RichText::new("Open Note →").color(t.accent_text)).clicked() {
                                *event_to_open = Some(ev.id.clone());
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        }
    });
}

/// Render the previous timeline list of existing daily notes.
fn render_timeline_list(
    ui: &mut Ui,
    state: &AppState,
    t: &Tokens,
    date_to_open: &mut Option<NaiveDate>,
) {
    // Date Navigator Banner
    let card = theme::card_frame(t, false).inner_margin(Margin::symmetric(18, 14));

    card.show(ui, |ui| {
        ui.horizontal(|ui| {
            let date_str = state.selected_daily_date.format("%A, %B %-d, %Y").to_string();
            ui.label(RichText::new(date_str).strong().size(15.0).color(t.accent_text));

            if state.selected_daily_date == Local::now().date_naive() {
                ui.add_space(4.0);
                theme::pill(ui, "Today", t.accent_text, t.accent_soft);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Open Note").strong().color(Color32::WHITE),
                        )
                        .fill(t.accent)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    )
                    .clicked()
                {
                    *date_to_open = Some(state.selected_daily_date);
                }
            });
        });
    });

    ui.add_space(20.0);
    ui.horizontal(|ui| {
        theme::section_label(ui, t, "Recent Daily Notes Timeline");
    });
    ui.add_space(6.0);

    let existing = state.daily_config.existing_dates(&state.vault);
    if existing.is_empty() {
        Frame::NONE
            .fill(t.card)
            .stroke(Stroke::new(1.0, t.card_border))
            .corner_radius(CornerRadius::same(t.radius as u8))
            .inner_margin(Margin::same(24))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No daily notes created yet.").color(t.text_dim).size(14.0));
                    ui.add_space(6.0);
                    ui.label(RichText::new("Use the calendar above or press Today to begin!").color(t.text_faint).size(12.0));
                });
            });
    } else {
        let mut dates: Vec<_> = existing.into_iter().collect();
        dates.sort_by(|a, b| b.cmp(a));

        for date in dates {
            let id = state.daily_config.id_for(date);
            let is_today = date == Local::now().date_naive();
            let is_active = state.active_note.as_ref() == Some(&id);

            let row_card = theme::card_frame(t, is_active).inner_margin(Margin::symmetric(14, 10));

            row_card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(date.format("%Y-%m-%d").to_string()).strong().size(13.5).color(t.text));
                    ui.label(RichText::new(date.format("%A").to_string()).color(t.text_dim).size(12.5));

                    if is_today {
                        theme::pill(ui, "Today", t.accent_text, t.accent_soft);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::ghost_button(ui, t, RichText::new("Open →").color(t.accent_text)).clicked() {
                            *date_to_open = Some(date);
                        }
                    });
                });
            });
            ui.add_space(6.0);
        }
    }
}
