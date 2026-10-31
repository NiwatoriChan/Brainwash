//! Vault switcher modal dialog.

use std::path::PathBuf;
use egui::{Color32, CornerRadius, Frame, Id, Margin, RichText, Stroke};

use crate::plugins::PluginContext;
use crate::theme::{self, Tokens};

pub struct VaultSwitcherState {
    pub is_open: bool,
    pub input_path: String,
}

impl Default for VaultSwitcherState {
    fn default() -> Self {
        Self {
            is_open: false,
            input_path: String::new(),
        }
    }
}

pub fn render_vault_switcher(
    ctx_egui: &egui::Context,
    switcher_state: &mut VaultSwitcherState,
    ctx: &mut PluginContext,
    switch_vault_req: &mut Option<PathBuf>,
) {
    if !switcher_state.is_open {
        return;
    }

    let t = Tokens::get(ctx_egui);
    let modal = egui::Modal::new(Id::unique("vault_switcher_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let mut should_close = false;

    let modal_resp = modal.show(ctx_egui, |ui| {
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
        let current_path = ctx.vault.root().display().to_string();
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
                egui::TextEdit::singleline(&mut switcher_state.input_path)
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
                let input = switcher_state.input_path.trim();
                if !input.is_empty() {
                    *switch_vault_req = Some(PathBuf::from(input));
                    should_close = true;
                }
            }
        });

        ui.add_space(14.0);

        // Recent vaults list
        if !ctx.settings.recent_vaults.is_empty() {
            ui.label(RichText::new("RECENT VAULTS").color(t.text_faint).size(11.0).strong());
            ui.add_space(4.0);

            let card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(8, 4));
            card.show(ui, |ui| {
                for recent in &ctx.settings.recent_vaults {
                    let is_current = recent == &current_path;
                    let folder_name = PathBuf::from(recent)
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| recent.clone());

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if is_current { "✓" } else { "📁" }).color(if is_current { t.success } else { t.text_dim }));
                        let btn = egui::Button::new(RichText::new(&folder_name).strong().color(if is_current { t.accent_text } else { t.text }))
                            .frame(false);
                        if ui.add(btn).clicked() && !is_current {
                            *switch_vault_req = Some(PathBuf::from(recent));
                            should_close = true;
                        }
                        ui.label(RichText::new(recent).color(t.text_faint).size(11.0));
                    });
                }
            });
        }
    });

    if modal_resp.should_close() || should_close {
        switcher_state.is_open = false;
        switcher_state.input_path.clear();
    }
}
