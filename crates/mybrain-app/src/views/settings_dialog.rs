//! Settings modal dialog for Brainwash.
//!
//! Provides configuration for:
//! - Themes (Studio, Studio Light, Pro, Solarized Dark, Adwaita Dark)
//! - Font family selection & UI font scaling
//! - Editor font size
//! - Vault defaults (Daily notes folder, Events folder, Canvas snap-to-grid)

use egui::{Color32, CornerRadius, Id, Margin, RichText, Stroke, Ui};

use crate::state::AppState;
use crate::theme::{self, ThemeKind, Tokens};

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_settings_dialog {
        return;
    }

    let t = Tokens::get(ctx);
    let modal = egui::Modal::new(Id::unique("settings_modal_dialog"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let mut should_close = false;
    let mut theme_changed = None;

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(520.0);

        // Header
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Settings").color(t.text).strong().size(18.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(10.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(10.0);

        egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
            // Section 1: Appearance & Theme
            theme::section_label(ui, &t, "Appearance");
            ui.add_space(4.0);

            let card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(14, 10));
            card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Color Theme").color(t.text).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("settings_theme_picker")
                            .selected_text(state.theme_kind.label())
                            .show_ui(ui, |ui| {
                                for kind in ThemeKind::ALL {
                                    if ui.selectable_label(state.theme_kind == kind, kind.label()).clicked() {
                                        theme_changed = Some(kind);
                                    }
                                }
                            });
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("UI Scaling").color(t.text).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut scale = state.settings.ui_scale;
                        if ui.add(egui::Slider::new(&mut scale, 0.80..=1.40).step_by(0.05).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                            state.settings.ui_scale = scale;
                            ctx.set_zoom_factor(scale);
                            state.save_settings();
                        }
                    });
                });
            });

            ui.add_space(12.0);

            // Section 2: Typography & Fonts
            theme::section_label(ui, &t, "Typography");
            ui.add_space(4.0);

            let card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(14, 10));
            card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Editor Font Size").color(t.text).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut size = state.settings.editor_font_size;
                        if ui.add(egui::Slider::new(&mut size, 11.0..=22.0).step_by(1.0).suffix(" px")).changed() {
                            state.settings.editor_font_size = size;
                            state.save_settings();
                        }
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Font Family").color(t.text).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let fonts = ["System Default", "DejaVu Sans", "Noto Sans", "Unifont"];
                        egui::ComboBox::from_id_salt("settings_font_picker")
                            .selected_text(&state.settings.font_family)
                            .show_ui(ui, |ui| {
                                for f in fonts {
                                    if ui.selectable_label(state.settings.font_family == f, f).clicked() {
                                        state.settings.font_family = f.to_owned();
                                        state.save_settings();
                                    }
                                }
                            });
                    });
                });
            });

            ui.add_space(12.0);

            // Section 3: Vault & Folders
            theme::section_label(ui, &t, "Vault & Folders");
            ui.add_space(4.0);

            let card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(14, 10));
            card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Daily Notes Folder").color(t.text).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::TextEdit::singleline(&mut state.settings.daily_folder).desired_width(120.0)).changed() {
                            state.daily_config.folder = state.settings.daily_folder.clone();
                            state.save_settings();
                        }
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Events Folder").color(t.text).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::TextEdit::singleline(&mut state.settings.events_folder).desired_width(120.0)).changed() {
                            state.event_store.folder = state.settings.events_folder.clone();
                            state.save_settings();
                        }
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Canvas Snap to Grid").color(t.text).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.checkbox(&mut state.settings.canvas_snap_to_grid, "").changed() {
                            state.canvas_state.snap_to_grid = state.settings.canvas_snap_to_grid;
                            state.save_settings();
                        }
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Vault Path").color(t.text_dim).size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let path_str = state.vault.root().display().to_string();
                        ui.label(RichText::new(path_str).monospace().size(11.0).color(t.text_faint));
                    });
                });
            });

            ui.add_space(14.0);

            // Shortcuts reference
            theme::section_label(ui, &t, "Keyboard Shortcuts");
            ui.add_space(4.0);
            let sc_card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(14, 10));
            sc_card.show(ui, |ui| {
                let row = |ui: &mut Ui, key: &str, desc: &str| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(desc).color(t.text_dim).size(12.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(key).monospace().size(11.0).color(t.accent_text));
                        });
                    });
                };
                row(ui, "Ctrl + P", "Command Palette / Quick Switcher");
                row(ui, "Ctrl + N", "New Note");
                row(ui, "Ctrl + Q", "Quick Note Capture");
                row(ui, "Ctrl + B", "Toggle Sidebar");
                row(ui, "Ctrl + ,", "Settings");
            });
        });

        ui.add_space(12.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(8.0);

        // Footer Done button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Done").strong().color(Color32::WHITE))
                        .fill(t.accent)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                )
                .clicked()
            {
                should_close = true;
            }
        });
    });

    if let Some(kind) = theme_changed {
        state.theme_kind = kind;
        state.settings.theme = format!("{kind:?}");
        theme::apply_theme(ctx, kind);
        state.save_settings();
    }

    if modal_resp.should_close() || should_close {
        state.show_settings_dialog = false;
    }
}
