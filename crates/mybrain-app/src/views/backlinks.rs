//! Contextual Backlinks, Outgoing links, and Unresolved links panel.

use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::NoteId;

use crate::state::AppState;
use crate::theme::{
    ACCENT_CYAN, ACCENT_PURPLE_LIGHT, ACCENT_RED, BG_CARD, BORDER_COLOR,
    TEXT_FAINT, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("🔗 Connections").color(TEXT_PRIMARY).strong().size(16.0));
    });

    ui.separator();

    let Some(active_id) = state.active_note.clone() else {
        ui.label(RichText::new("Select a note to inspect connections").color(TEXT_MUTED));
        return;
    };

    let mut note_to_open: Option<NoteId> = None;
    let mut note_to_create: Option<String> = None;

    ScrollArea::vertical().id_salt("backlinks_scroll").show(ui, |ui| {
        // 1. Backlinks (incoming links)
        let backlinks: Vec<_> = state.link_index.backlinks(&active_id).to_vec();
        ui.label(RichText::new(format!("Incoming Backlinks ({})", backlinks.len())).strong().color(ACCENT_PURPLE_LIGHT).size(13.0));

        if backlinks.is_empty() {
            ui.label(RichText::new("No other notes link here yet.").color(TEXT_FAINT).size(12.0));
        } else {
            for bl in backlinks {
                let source_id = bl.source.clone();
                Frame::NONE
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.0, BORDER_COLOR))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("📄").size(12.0));
                            if ui.button(RichText::new(source_id.name()).strong().color(TEXT_PRIMARY)).clicked() {
                                note_to_open = Some(source_id);
                            }
                        });
                        if !bl.context.is_empty() {
                            ui.label(RichText::new(&bl.context).color(TEXT_MUTED).size(11.0).italics());
                        }
                    });
                ui.add_space(4.0);
            }
        }

        ui.add_space(12.0);
        ui.separator();

        // 2. Outgoing Links
        let outgoing: Vec<_> = state.link_index.outgoing(&active_id).to_vec();
        ui.label(RichText::new(format!("Outgoing Links ({})", outgoing.len())).strong().color(ACCENT_CYAN).size(13.0));

        if outgoing.is_empty() {
            ui.label(RichText::new("No outgoing [[wiki links]] in this note.").color(TEXT_FAINT).size(12.0));
        } else {
            for out in outgoing {
                let target = out.link.target.clone();
                let is_resolved = out.resolved.is_some();

                Frame::NONE
                    .fill(BG_CARD)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let icon = if is_resolved { "🔗" } else { "❓" };
                            ui.label(RichText::new(icon).size(12.0));

                            let text = RichText::new(out.link.display_text()).color(if is_resolved {
                                ACCENT_PURPLE_LIGHT
                            } else {
                                ACCENT_RED
                            });

                            if ui.button(text).clicked() {
                                note_to_create = Some(target);
                            }

                            if !is_resolved {
                                ui.label(RichText::new("(uncreated)").color(TEXT_FAINT).size(10.0));
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
            ui.label(RichText::new("Unresolved Links Across Vault").strong().color(TEXT_MUTED).size(12.0));
            for (target, ref_count) in unresolved {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("• [[{target}]]")).color(TEXT_MUTED).size(11.0));
                    ui.label(RichText::new(format!("({ref_count} ref)")).color(TEXT_FAINT).size(10.0));
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
