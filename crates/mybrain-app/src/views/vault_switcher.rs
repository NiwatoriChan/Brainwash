//! Vault management and switcher dialog.
//!
//! Allows switching to another vault folder, viewing recent vaults,
//! or creating a new vault on disk without restarting the application.

use std::path::PathBuf;
use egui::{Color32, CornerRadius, Frame, Id, Margin, RichText, Stroke};

use crate::state::AppState;
use crate::theme::{self, Tokens};

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_vault_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let modal = egui::Modal::new(Id::unique("vault_switcher_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let mut should_close = false;
    let mut switch_to_path: Option<PathBuf> = None;

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(480.0);

        // Header
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Vault Switcher").color(t.text).strong().size(17.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(8.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(10.0);

        // Current active vault
        ui.label(RichText::new("CURRENT VAULT").color(t.text_faint).size(11.0).strong());
        let current_path = state.vault.root().display().to_string();
        Frame::NONE
            .fill(t.field)
            .corner_radius(CornerRadius::same(t.radius_sm as u8))
            .stroke(Stroke::new(1.0, t.field_border))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📁").size(14.0));
                    ui.label(RichText::new(&current_path).color(t.text).monospace().size(12.0));
                });
            });

        ui.add_space(12.0);

        // Open another vault path
        ui.label(RichText::new("OPEN FOLDER AS VAULT").color(t.text_faint).size(11.0).strong());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.vault_path_input)
                    .hint_text("/path/to/notes/folder")
                    .desired_width(ui.available_width() - 80.0),
            );
            if ui
                .add(
                    egui::Button::new(RichText::new("Open").strong().color(Color32::WHITE))
                        .fill(t.accent)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                )
                .clicked()
            {
                let input = state.vault_path_input.trim();
                if !input.is_empty() {
                    switch_to_path = Some(PathBuf::from(input));
                }
            }
        });

        ui.add_space(14.0);

        // Recent vaults list
        if !state.settings.recent_vaults.is_empty() {
            ui.label(RichText::new("RECENT VAULTS").color(t.text_faint).size(11.0).strong());
            ui.add_space(4.0);

            let card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(8, 4));
            card.show(ui, |ui| {
                for recent in &state.settings.recent_vaults {
                    let is_current = recent == &current_path;
                    let folder_name = PathBuf::from(recent)
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| recent.clone());

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if is_current { "✓" } else { "📁" }).color(if is_current { t.success } else { t.text_dim }));
                        let btn = ui.selectable_label(is_current, RichText::new(&folder_name).color(t.text));
                        if btn.clicked() && !is_current {
                            switch_to_path = Some(PathBuf::from(recent));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(recent).color(t.text_faint).monospace().size(10.5));
                        });
                    });
                }
            });
        }
    });

    if let Some(path) = switch_to_path {
        if let Err(e) = state.switch_vault(path) {
            state.set_status(format!("Error opening vault: {e}"));
        } else {
            should_close = true;
        }
    }

    if modal_resp.should_close() || should_close {
        state.show_vault_dialog = false;
    }
}
