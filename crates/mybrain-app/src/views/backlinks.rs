//! Contextual Backlinks, Outgoing links, and Unresolved links panel.

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::NoteId;

use crate::state::AppState;
use crate::theme::Tokens;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    ui.horizontal(|ui| {
        ui.heading(RichText::new("Connections").color(t.text).strong().size(16.0));
    });

    ui.separator();

    let Some(active_id) = state.active_note.clone() else {
        ui.label(RichText::new("Select a note to inspect connections").color(t.text_dim));
        return;
    };

    let mut note_to_open: Option<NoteId> = None;
    let mut note_to_create: Option<String> = None;

    ScrollArea::vertical().id_salt("backlinks_scroll").show(ui, |ui| {
        // 1. Backlinks (incoming links)
        let backlinks: Vec<_> = state.link_index.backlinks(&active_id).to_vec();
        ui.label(RichText::new(format!("Incoming Backlinks ({})", backlinks.len())).strong().color(t.accent_text).size(13.0));

        if backlinks.is_empty() {
            ui.label(RichText::new("No other notes link here yet.").color(t.text_faint).size(12.0));
        } else {
            for bl in backlinks {
                let source_id = bl.source.clone();
                Frame::NONE
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.card_border))
                    .corner_radius(CornerRadius::same(t.radius_sm as u8))
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("←").strong().color(t.accent_text));
                            if ui.button(RichText::new(source_id.name()).strong().color(t.text)).clicked() {
                                note_to_open = Some(source_id);
                            }
                        });
                        if !bl.context.is_empty() {
                            ui.label(RichText::new(&bl.context).color(t.text_dim).size(11.0).italics());
                        }
                    });
                ui.add_space(4.0);
            }
        }

        ui.add_space(12.0);
        ui.separator();

        // 2. Outgoing Links
        let outgoing: Vec<_> = state.link_index.outgoing(&active_id).to_vec();
        ui.label(RichText::new(format!("Outgoing Links ({})", outgoing.len())).strong().color(t.accent).size(13.0));

        if outgoing.is_empty() {
            ui.label(RichText::new("No outgoing [[wiki links]] in this note.").color(t.text_faint).size(12.0));
        } else {
            for out in outgoing {
                let target = out.link.target.clone();
                let is_resolved = out.resolved.is_some();

                Frame::NONE
                    .fill(t.card)
                    .corner_radius(CornerRadius::same(t.radius_sm as u8))
                    .inner_margin(Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let icon = if is_resolved { "→" } else { "?" };
                            ui.label(RichText::new(icon).strong().color(if is_resolved { t.accent_text } else { t.warning }));

                            let text = RichText::new(out.link.display_text()).color(if is_resolved {
                                t.accent_text
                            } else {
                                t.danger
                            });

                            if ui.button(text).clicked() {
                                note_to_create = Some(target);
                            }

                            if !is_resolved {
                                ui.label(RichText::new("(uncreated)").color(t.text_faint).size(10.0));
                            }
                        });
                    });
                ui.add_space(2.0);
            }
        }

        ui.add_space(12.0);
        ui.separator();

        // 3. Vault Unresolved Links
        let unresolved: Vec<_> = state
            .link_index
            .unresolved()
            .iter()
            .map(|(t, r)| (t.clone(), r.len()))
            .collect();

        if !unresolved.is_empty() {
            ui.label(RichText::new("Unresolved Links Across Vault").strong().color(t.text_dim).size(12.0));
            for (target, ref_count) in unresolved {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("• [[{target}]]")).color(t.text_dim).size(11.0));
                    ui.label(RichText::new(format!("({ref_count} ref)")).color(t.text_faint).size(10.0));
                    if ui.small_button("+ Create").clicked() {
                        note_to_create = Some(target);
                    }
                });
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
