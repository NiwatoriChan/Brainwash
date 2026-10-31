//! Calendar Events Agenda plugin for Brainwash.

use chrono::Local;
use egui::{Frame, Margin, RichText, ScrollArea, Ui};
use mybrain_core::events::EventStore;
use mybrain_core::events_bus::AppEvent;

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::{self, Tokens};

pub struct EventsPlugin;

impl EventsPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EventsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for EventsPlugin {
    fn id(&self) -> &'static str {
        "core.events"
    }

    fn name(&self) -> &'static str {
        "Events View"
    }

    fn description(&self) -> &'static str {
        "Agenda and calendar view of scheduled events and appointments embedded in notes"
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "🗓",
            tooltip: "Events & Agenda",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::MainWorkspace,
            priority: 70,
        })
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            id: "open_events".into(),
            title: "Open Events Agenda View".into(),
            shortcut: None,
            plugin_id: "core.events",
        }]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        if command_id == "open_events" {
            ctx.activate_plugin("core.events");
        }
    }

    fn render_main_view(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());
        let store = EventStore::new(&ctx.settings.events_folder);
        let events = store.all(ctx.vault);

        ScrollArea::vertical()
            .id_salt("events_view_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::NONE
                    .inner_margin(Margin::symmetric(28, 20))
                    .show(ui, |ui| {
                        let avail_w = ui.available_width();
                        let max_w = avail_w.min(theme::READABLE_WIDTH);
                        let side_pad = ((avail_w - max_w) * 0.5).max(0.0);

                        ui.horizontal(|ui| {
                            if side_pad > 0.0 {
                                ui.add_space(side_pad);
                            }
                            ui.vertical(|ui| {
                                ui.set_max_width(max_w);

                                // Header Card
                                let header_card = theme::card_frame(&t, false)
                                    .inner_margin(Margin::symmetric(18, 14));

                                header_card.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.heading(RichText::new("Events Agenda").color(t.text).strong().size(16.0));
                                        ui.label(RichText::new(format!("({} scheduled)", events.len())).color(t.text_dim).size(12.0));

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if theme::ghost_button(ui, &t, RichText::new("Calendar View →").color(t.accent_text)).clicked() {
                                                ctx.activate_plugin("core.daily");
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
                                                    RichText::new("Add an ```event block or frontmatter to any note to track it here.")
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
                                                            ctx.emit(AppEvent::RequestOpenDaily(event.date));
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
                                                        ctx.open_note(event.id.clone());
                                                        ctx.activate_plugin("core.markdown");
                                                    }
                                                });
                                            });
                                        });

                                        ui.add_space(6.0);
                                    }
                                }
                            });
                        });
                        ui.add_space(20.0);
                    });
            });
    }
}
