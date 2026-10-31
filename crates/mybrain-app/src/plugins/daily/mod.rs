//! Daily notes calendar and timeline navigator plugin for Brainwash.

use std::collections::BTreeMap;

use chrono::{Datelike, Local, NaiveDate};
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, Frame, Margin, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui,
};
use mybrain_core::daily::DailyNotes;
use mybrain_core::events::{Event, EventStore};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::index::LinkIndex;
use mybrain_core::note::NoteId;
use mybrain_core::vault::Vault;

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::{self, Tokens};

pub struct DailyPlugin {
    pub selected_date: NaiveDate,
    pub view_date: NaiveDate,
}

impl DailyPlugin {
    pub fn new() -> Self {
        let today = Local::now().date_naive();
        Self {
            selected_date: today,
            view_date: today,
        }
    }

    pub fn shift_month(&mut self, delta: i32) {
        let mut year = self.view_date.year();
        let mut month = self.view_date.month() as i32 + delta;
        while month > 12 {
            month -= 12;
            year += 1;
        }
        while month < 1 {
            month += 12;
            year -= 1;
        }
        if let Some(d) = NaiveDate::from_ymd_opt(year, month as u32, 1) {
            self.view_date = d;
        }
    }

    pub fn open_daily_date(&mut self, date: NaiveDate, ctx: &mut PluginContext) -> NoteId {
        let daily = DailyNotes {
            folder: ctx.settings.daily_folder.clone(),
            ..Default::default()
        };
        let id = daily.id_for(date);
        if !ctx.vault.contains(&id) {
            let rendered = daily.render_template(date);
            let _ = ctx.vault.create(id.clone(), &rendered);
            *ctx.index = LinkIndex::build(ctx.vault);
        }
        self.selected_date = date;
        self.view_date = date;
        ctx.open_note(id.clone());
        ctx.activate_plugin("core.markdown");
        id
    }

    pub fn sync_events_to_daily_note(date: NaiveDate, vault: &mut Vault, daily_folder: &str, events_folder: &str) -> usize {
        let store = EventStore::new(events_folder);
        let events = store.on(vault, date);
        if events.is_empty() {
            return 0;
        }

        let daily = DailyNotes {
            folder: daily_folder.to_string(),
            ..Default::default()
        };
        let daily_id = match daily.open_or_create(vault, date) {
            Ok(id) => id,
            Err(_) => return 0,
        };

        if let Some(daily_note) = vault.get(&daily_id) {
            let mut content = daily_note.content.clone();
            let mut added = 0;
            for ev in events {
                if !content.contains(&ev.title) {
                    let bullet = ev.to_markdown_bullet();
                    if !content.ends_with('\n') && !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&bullet);
                    added += 1;
                }
            }
            if added > 0 {
                let _ = vault.save(&daily_id, &content);
            }
            return added;
        }
        0
    }
}

impl Default for DailyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for DailyPlugin {
    fn id(&self) -> &'static str {
        "core.daily"
    }

    fn name(&self) -> &'static str {
        "Daily Notes"
    }

    fn description(&self) -> &'static str {
        "Calendar view and timeline navigation for daily notes and scheduled events"
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "📅",
            tooltip: "Daily Notes & Calendar (Ctrl+2)",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::MainWorkspace,
            priority: 40,
        })
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "open_today".into(),
                title: "Open Today's Daily Note".into(),
                shortcut: None,
                plugin_id: "core.daily",
            },
            PluginCommand {
                id: "open_calendar".into(),
                title: "Open Calendar View".into(),
                shortcut: Some("Ctrl+2".into()),
                plugin_id: "core.daily",
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        match command_id {
            "open_today" => {
                let today = Local::now().date_naive();
                self.open_daily_date(today, ctx);
            }
            "open_calendar" => {
                ctx.activate_plugin("core.daily");
            }
            _ => {}
        }
    }

    fn on_event(&mut self, event: &AppEvent, ctx: &mut PluginContext) {
        if let AppEvent::RequestOpenDaily(date) = event {
            self.open_daily_date(*date, ctx);
        }
    }

    fn render_main_view(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());
        let mut date_to_open = None;
        let mut sync_date = None;

        ScrollArea::vertical()
            .id_salt("daily_view_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::NONE
                    .inner_margin(Margin::symmetric(28, 16))
                    .show(ui, |ui| {
                        let avail_w = ui.available_width();
                        let max_w = avail_w.min(theme::READABLE_WIDTH + 100.0);
                        let side_pad = ((avail_w - max_w) * 0.5).max(0.0);

                        ui.horizontal(|ui| {
                            if side_pad > 0.0 {
                                ui.add_space(side_pad);
                            }
                            ui.vertical(|ui| {
                                ui.set_max_width(max_w);

                                // Top Action Bar
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.heading(RichText::new("Calendar & Daily Notes").strong().size(18.0).color(t.text));
                                        ui.label(RichText::new("Click any date to open or create its daily note").color(t.text_dim).size(12.0));
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Open Today's Note →")
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(t.accent)
                                                .stroke(Stroke::NONE)
                                                .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                                            )
                                            .on_hover_text("Open or create today's daily note")
                                            .clicked()
                                        {
                                            date_to_open = Some(Local::now().date_naive());
                                        }

                                        if theme::ghost_button(
                                            ui,
                                            &t,
                                            RichText::new("Jump to Today").color(t.accent_text).strong(),
                                        )
                                        .on_hover_text("View current month in calendar")
                                        .clicked()
                                        {
                                            let today = Local::now().date_naive();
                                            self.selected_date = today;
                                            self.view_date = today;
                                        }
                                    });
                                });

                                ui.add_space(14.0);

                                // Month Navigation Header Card
                                let nav_card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(16, 10));
                                nav_card.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if theme::ghost_button(ui, &t, RichText::new("← Prev Month").color(t.text_dim)).clicked() {
                                            self.shift_month(-1);
                                        }

                                        ui.add_space(8.0);

                                        let month_title = self.view_date.format("%B %Y").to_string();
                                        ui.label(RichText::new(month_title).strong().size(16.0).color(t.text));

                                        if self.view_date.year() == Local::now().year()
                                            && self.view_date.month() == Local::now().month()
                                        {
                                            ui.add_space(4.0);
                                            theme::pill(ui, "Current Month", t.accent_text, t.accent_soft);
                                        }

                                        ui.add_space(8.0);

                                        if theme::ghost_button(ui, &t, RichText::new("Next Month →").color(t.text_dim)).clicked() {
                                            self.shift_month(1);
                                        }

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let sel_str = format!("Selected: {}", self.selected_date.format("%a, %b %-d"));
                                            ui.label(RichText::new(sel_str).size(12.5).color(t.accent_text));
                                        });
                                    });
                                });

                                ui.add_space(10.0);

                                // Pre-query daily notes and events for this month
                                let daily = DailyNotes {
                                    folder: ctx.settings.daily_folder.clone(),
                                    ..Default::default()
                                };
                                let existing_daily = daily.existing_dates(ctx.vault);

                                let store = EventStore::new(&ctx.settings.events_folder);
                                let year = self.view_date.year();
                                let month = self.view_date.month();
                                let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(self.view_date);
                                let lead_padding = first_of_month.weekday().num_days_from_monday();

                                let next_month = if month == 12 {
                                    NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
                                } else {
                                    NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
                                };
                                let days_in_month = (next_month - first_of_month).num_days() as u32;

                                let month_events = store.between(ctx.vault, first_of_month, next_month - chrono::Duration::days(1));

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
                                                let (rect, _) = ui.allocate_exact_size(vec2(col_w, row_h), Sense::hover());
                                                ui.painter().rect_filled(rect, CornerRadius::same(t.radius_sm as u8), Color32::TRANSPARENT);
                                            } else {
                                                let day_num = (slot - lead_padding + 1) as u32;
                                                let cell_date = NaiveDate::from_ymd_opt(year, month, day_num).unwrap();
                                                let is_today = cell_date == Local::now().date_naive();
                                                let is_selected = cell_date == self.selected_date;
                                                let has_daily = existing_daily.contains(&cell_date);
                                                let day_events = events_by_day.get(&day_num);

                                                let (rect, resp) = ui.allocate_exact_size(vec2(col_w, row_h), Sense::click());

                                                if resp.clicked() {
                                                    self.selected_date = cell_date;
                                                    date_to_open = Some(cell_date);
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

                                                // Top row of cell
                                                let top_left = rect.left_top() + vec2(7.0, 6.0);
                                                let day_color = if is_today {
                                                    t.accent_text
                                                } else if is_selected {
                                                    t.text
                                                } else {
                                                    t.text_dim
                                                };

                                                if is_today {
                                                    let pill_rect = Rect::from_min_size(top_left - vec2(2.0, 1.0), vec2(22.0, 18.0));
                                                    ui.painter().rect_filled(pill_rect, CornerRadius::same(4), t.accent_soft);
                                                }

                                                ui.painter().text(
                                                    top_left + vec2(9.0, 8.0),
                                                    egui::Align2::CENTER_CENTER,
                                                    day_num.to_string(),
                                                    egui::FontId::proportional(12.5),
                                                    day_color,
                                                );

                                                if has_daily {
                                                    let dot_pos = rect.right_top() + vec2(-10.0, 12.0);
                                                    ui.painter().circle_filled(dot_pos, 3.5, t.accent);
                                                }

                                                if let Some(events) = day_events {
                                                    let mut y = rect.top() + 26.0;
                                                    for ev in events.iter().take(2) {
                                                        if y + 14.0 > rect.bottom() - 4.0 {
                                                            break;
                                                        }
                                                        ui.painter().circle_filled(pos2(rect.left() + 9.0, y + 6.0), 2.5, t.warning);

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
                                                }

                                                let hint = if has_daily {
                                                    format!("Open daily note for {}", cell_date.format("%A, %B %-d"))
                                                } else {
                                                    format!("Create and open daily note for {}", cell_date.format("%A, %B %-d"))
                                                };
                                                resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(hint);
                                            }
                                        }
                                    });
                                    ui.add_space(col_gap);
                                }

                                ui.add_space(16.0);

                                // Selected Day Detail Card
                                let is_today = self.selected_date == Local::now().date_naive();
                                let has_daily = ctx.vault.get(&daily.id_for(self.selected_date)).is_some();
                                let day_events = store.on(ctx.vault, self.selected_date);

                                let panel = theme::card_frame(&t, false).inner_margin(Margin::same(18));
                                panel.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let title_str = self.selected_date.format("%A, %B %-d, %Y").to_string();
                                        ui.heading(RichText::new(title_str).size(16.0).strong().color(t.text));

                                        if is_today {
                                            ui.add_space(4.0);
                                            theme::pill(ui, "Today", t.accent_text, t.accent_soft);
                                        }

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(if has_daily { "Open Daily Note →" } else { "+ Create Daily Note" })
                                                            .strong()
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(t.accent)
                                                    .stroke(Stroke::NONE)
                                                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                                                )
                                                .clicked()
                                            {
                                                date_to_open = Some(self.selected_date);
                                            }

                                            if !day_events.is_empty() {
                                                if theme::ghost_button(ui, &t, RichText::new("Sync Events to Note").color(t.accent_text)).clicked() {
                                                    sync_date = Some(self.selected_date);
                                                }
                                            }
                                        });
                                    });

                                    ui.add_space(10.0);
                                    if day_events.is_empty() {
                                        ui.label(RichText::new("No scheduled events for this day.").color(t.text_faint).size(12.0));
                                    } else {
                                        for ev in &day_events {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("🔔").color(t.warning).size(12.0));
                                                ui.label(RichText::new(&ev.title).strong().color(t.text).size(13.0));
                                                if let Some(st) = ev.start {
                                                    ui.label(RichText::new(st.format("%H:%M").to_string()).color(t.text_dim).size(11.5));
                                                }
                                            });
                                        }
                                    }
                                });
                            });
                        });
                        ui.add_space(24.0);
                    });
            });

        if let Some(d) = date_to_open {
            self.open_daily_date(d, ctx);
        }

        if let Some(d) = sync_date {
            let added = Self::sync_events_to_daily_note(d, ctx.vault, &ctx.settings.daily_folder, &ctx.settings.events_folder);
            if added > 0 {
                ctx.toast(format!("Synced {added} event(s) to daily note"));
            } else {
                ctx.toast("All events already synced to daily note");
            }
        }
    }
}
