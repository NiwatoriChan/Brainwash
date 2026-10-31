//! Contextual Backlinks, Outgoing links, and Unresolved links panel plugin.

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::index::LinkIndex;
use mybrain_core::NoteId;

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::{self, Tokens};

pub struct BacklinksPlugin;

impl BacklinksPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BacklinksPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for BacklinksPlugin {
    fn id(&self) -> &'static str {
        "core.backlinks"
    }

    fn name(&self) -> &'static str {
        "Backlinks Panel"
    }

    fn description(&self) -> &'static str {
        "Inspect incoming backlinks, outgoing wiki-links, and unresolved links for the active note"
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "🔗",
            tooltip: "Toggle Connections Panel",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::SidePanel,
            priority: 80,
        })
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            id: "toggle_backlinks".into(),
            title: "Toggle Connections & Backlinks Panel".into(),
            shortcut: None,
            plugin_id: "core.backlinks",
        }]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        if command_id == "toggle_backlinks" {
            ctx.toggle_side_panel("core.backlinks");
        }
    }

    fn render_side_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());

        // Header
        ui.horizontal(|ui| {
            theme::section_label(ui, &t, "Connections");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t)
                    .on_hover_text("Hide connections panel")
                    .clicked()
                {
                    ctx.toggle_side_panel("core.backlinks");
                }
            });
        });

        ui.add_space(4.0);

        let Some(active_id) = ctx.active_note.cloned() else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Select a note to inspect connections").color(t.text_faint).size(12.0));
            });
            return;
        };

        let mut note_to_open: Option<NoteId> = None;
        let mut note_to_create: Option<String> = None;

        ScrollArea::vertical().id_salt("backlinks_scroll").show(ui, |ui| {
            // 1. Backlinks (incoming links)
            let backlinks: Vec<_> = ctx.index.backlinks(&active_id).to_vec();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Incoming ({})", backlinks.len())).strong().color(t.accent_text).size(12.0));
            });
            ui.add_space(2.0);

            if backlinks.is_empty() {
                ui.label(RichText::new("No incoming backlinks").color(t.text_faint).size(11.5));
            } else {
                for bl in backlinks {
                    let source_id = bl.source.clone();
                    let source_note = ctx.vault.get(&source_id);
                    let source_fm = source_note.map(|n| n.frontmatter());
                    let source_color = source_fm.as_ref().and_then(|fm| fm.get("color")).and_then(crate::theme::parse_color_str);
                    let source_emoji = source_fm.as_ref().and_then(|fm| fm.get("icon")).map(|s| s.to_string());
                    let is_daily = source_id.as_str().starts_with("Daily/");
                    let is_event = source_id.as_str().starts_with("Events/");

                    let dot_col = source_color.unwrap_or_else(|| {
                        if is_event { t.warning }
                        else if is_daily { t.accent }
                        else { theme::note_kind_color(source_id.as_str(), &t) }
                    });
                    let icon_str = if let Some(ref e) = source_emoji {
                        e.as_str()
                    } else if is_daily {
                        "📅"
                    } else if is_event {
                        "🔔"
                    } else {
                        "📝"
                    };

                    let card = theme::card_frame(&t, false)
                        .inner_margin(Margin::same(8));

                    card.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(icon_str).color(dot_col).size(12.5));
                            if ui.button(RichText::new(source_id.name()).strong().color(t.text)).clicked() {
                                note_to_open = Some(source_id);
                            }
                        });
                        if !bl.context.is_empty() {
                            ui.add_space(2.0);
                            ui.label(RichText::new(&bl.context).color(t.text_dim).size(11.0).italics());
                        }
                    });
                    ui.add_space(4.0);
                }
            }

            ui.add_space(10.0);
            theme::bottom_hairline(ui, ui.max_rect(), &t);
            ui.add_space(8.0);

            // 2. Outgoing Links
            let outgoing: Vec<_> = ctx.index.outgoing(&active_id).to_vec();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Outgoing ({})", outgoing.len())).strong().color(t.accent).size(12.0));
            });
            ui.add_space(2.0);

            if outgoing.is_empty() {
                ui.label(RichText::new("No outgoing [[links]] in this note").color(t.text_faint).size(11.5));
            } else {
                for out in outgoing {
                    let target = out.link.target.clone();
                    let is_resolved = out.resolved.is_some();
                    let resolved_id = out.resolved.as_ref();
                    let res_note = resolved_id.and_then(|id| ctx.vault.get(id));
                    let res_fm = res_note.map(|n| n.frontmatter());
                    let custom_color = res_fm.as_ref().and_then(|fm| fm.get("color")).and_then(crate::theme::parse_color_str);
                    let custom_emoji = res_fm.as_ref().and_then(|fm| fm.get("icon")).map(|s| s.to_string());
                    let is_daily = resolved_id.map(|id| id.as_str().starts_with("Daily/")).unwrap_or(false);
                    let is_event = resolved_id.map(|id| id.as_str().starts_with("Events/")).unwrap_or(false);

                    let dot_col = custom_color.unwrap_or_else(|| {
                        if is_event { t.warning }
                        else if is_daily { t.accent }
                        else if is_resolved { t.accent_text }
                        else { t.text_faint }
                    });
                    let icon_str = if let Some(ref e) = custom_emoji {
                        e.as_str()
                    } else if is_daily {
                        "📅"
                    } else if is_event {
                        "🔔"
                    } else if is_resolved {
                        "📝"
                    } else {
                        "❓"
                    };

                    Frame::NONE
                        .fill(t.card)
                        .stroke(Stroke::new(1.0, t.card_border))
                        .corner_radius(CornerRadius::same(t.radius_sm as u8))
                        .inner_margin(Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(icon_str).color(dot_col).size(12.5));

                                let text = RichText::new(out.link.display_text()).color(if is_resolved {
                                    t.text
                                } else {
                                    t.warning
                                });

                                if is_resolved {
                                    if ui.button(text).clicked() {
                                        if let Some(res) = &out.resolved {
                                            note_to_open = Some(res.clone());
                                        }
                                    }
                                } else {
                                    ui.label(text);
                                    if theme::ghost_button(ui, &t, RichText::new("+ Create").color(t.accent_text).size(10.5))
                                        .on_hover_text("Click to create this linked note")
                                        .clicked()
                                    {
                                        note_to_create = Some(target);
                                    }
                                }
                            });
                        });
                    ui.add_space(4.0);
                }
            }
        });

        if let Some(id) = note_to_open {
            ctx.open_note(id);
            ctx.activate_plugin("core.markdown");
        }

        if let Some(target) = note_to_create {
            if let Ok(id) = NoteId::new(&target) {
                let initial = format!("# {}\n\nCreated from connection link.\n", id.name());
                let _ = ctx.vault.create(id.clone(), &initial);
                *ctx.index = LinkIndex::build(ctx.vault);
                ctx.open_note(id);
                ctx.activate_plugin("core.markdown");
            }
        }
    }
}
