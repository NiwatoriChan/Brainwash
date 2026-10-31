//! Contextual Backlinks, Outgoing links, and Unresolved links panel.

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::NoteId;

use crate::state::AppState;
use crate::theme::{self, Tokens};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    // Panel Header
    ui.horizontal(|ui| {
        theme::section_label(ui, &t, "Connections");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::icon_close_button(ui, &t)
                .on_hover_text("Hide connections panel")
                .clicked()
            {
                state.show_backlinks = false;
            }
        });
    });

    ui.add_space(4.0);

    let Some(active_id) = state.active_note.clone() else {
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
        let backlinks: Vec<_> = state.link_index.backlinks(&active_id).to_vec();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Incoming ({})", backlinks.len())).strong().color(t.accent_text).size(12.0));
        });
        ui.add_space(2.0);

        if backlinks.is_empty() {
            ui.label(RichText::new("No incoming backlinks").color(t.text_faint).size(11.5));
        } else {
            for bl in backlinks {
                let source_id = bl.source.clone();
                let source_color = state.get_note_color(&source_id);
                let source_emoji = state.get_note_emoji(&source_id);
                let is_daily = state.is_daily_note(&source_id);
                let is_event = state.is_event_note(&source_id);

                let dot_col = source_color.unwrap_or_else(|| {
                    if is_event { t.warning }
                    else if is_daily { t.accent }
                    else { theme::note_kind_color(source_id.as_str(), &t) }
                });
                let icon_str = if let Some(e) = &source_emoji {
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
        let outgoing: Vec<_> = state.link_index.outgoing(&active_id).to_vec();
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
                let custom_color = resolved_id.and_then(|id| state.get_note_color(id));
                let custom_emoji = resolved_id.and_then(|id| state.get_note_emoji(id));
                let is_daily = resolved_id.map(|id| state.is_daily_note(id)).unwrap_or(false);
                let is_event = resolved_id.map(|id| state.is_event_note(id)).unwrap_or(false);

                let dot_col = custom_color.unwrap_or_else(|| {
                    if is_event { t.warning }
                    else if is_daily { t.accent }
                    else if is_resolved { t.accent_text }
                    else { t.text_faint }
                });
                let icon_str = if let Some(e) = &custom_emoji {
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
                                if ui.button(text).on_hover_text("Unresolved link - click to create").clicked() {
                                    note_to_create = Some(target);
                                }
                            }
                        });
                    });
                ui.add_space(3.0);
            }
        }

        ui.add_space(10.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(8.0);

        // 3. Vault Unresolved Links
        let unresolved: Vec<_> = state
            .link_index
            .unresolved()
            .iter()
            .map(|(t, r)| (t.clone(), r.len()))
            .collect();

        if !unresolved.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Unresolved in Vault").strong().color(t.text_dim).size(12.0));
            });
            ui.add_space(3.0);

            for (target, ref_count) in unresolved {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("• [[{target}]]")).color(t.text_dim).size(11.5));
                    ui.label(RichText::new(format!("({ref_count})")).color(t.text_faint).size(10.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::ghost_button(ui, &t, RichText::new("+ Create").color(t.accent_text).size(11.0)).clicked() {
                            note_to_create = Some(target);
                        }
                    });
                });
                ui.add_space(2.0);
            }
        }
    });

    if let Some(id) = note_to_open {
        state.open_note(id);
    }
    if let Some(target) = note_to_create {
        state.open_or_create(&target);
    }
}
