//! Settings modal dialog with General settings and dedicated Plugin Manager.

use std::path::Path;

use egui::{vec2, Color32, CornerRadius, Id, Margin, RichText, Sense, Stroke};
use mybrain_core::settings::{default_color_palette, ColorDef};

use crate::plugins::{PluginContext, PluginRegistry, PluginViewType};
use crate::theme::{self, ThemeKind, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Plugins,
}

pub struct SettingsModalState {
    pub active_tab: SettingsTab,
    pub new_color_hex: String,
    pub new_color_label: String,
}

impl Default for SettingsModalState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::General,
            new_color_hex: "#2d9cdb".into(),
            new_color_label: String::new(),
        }
    }
}

pub fn render_settings_modal(
    ctx_egui: &egui::Context,
    modal_state: &mut SettingsModalState,
    is_open: &mut bool,
    registry: &mut PluginRegistry,
    active_main: &mut Option<String>,
    active_side: &mut Option<String>,
    active_right: &mut Option<String>,
    ctx: &mut PluginContext,
    settings_path: &Path,
) {
    if !*is_open {
        return;
    }

    let t = Tokens::get(ctx_egui);
    let modal = egui::Modal::new(Id::unique("settings_modal_dialog"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let mut should_close = false;
    let mut theme_changed = None;
    let mut plugin_toggle_req: Option<(String, bool)> = None;

    let modal_resp = modal.show(ctx_egui, |ui| {
        ui.set_width(560.0);

        // Header
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Preferences & Plugins").color(t.text).strong().size(18.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(8.0);

        // Tab Switcher: General vs Plugins
        ui.horizontal(|ui| {
            if let Some(tab) = theme::segmented(
                ui,
                &t,
                modal_state.active_tab,
                &[
                    (SettingsTab::General, "General & Appearance"),
                    (SettingsTab::Plugins, "Plugins & Modules"),
                ],
            ) {
                modal_state.active_tab = tab;
            }
        });

        ui.add_space(8.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(10.0);

        egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
            match modal_state.active_tab {
                SettingsTab::General => {
                    // Section 1: Appearance & Theme
                    theme::section_label(ui, &t, "Appearance");
                    ui.add_space(4.0);

                    let card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(14, 10));
                    card.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Color Theme").color(t.text).size(13.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                egui::ComboBox::from_id_salt("settings_theme_picker")
                                    .selected_text(ctx.theme_kind.label())
                                    .show_ui(ui, |ui| {
                                        for kind in ThemeKind::ALL {
                                            if ui.selectable_label(*ctx.theme_kind == kind, kind.label()).clicked() {
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
                                let mut scale = ctx.settings.ui_scale;
                                if ui.add(egui::Slider::new(&mut scale, 0.80..=1.40).step_by(0.05).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                                    ctx.settings.ui_scale = scale;
                                    ctx_egui.set_zoom_factor(scale);
                                    let _ = ctx.settings.save_to_file(settings_path);
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
                                let mut size = ctx.settings.editor_font_size;
                                if ui.add(egui::Slider::new(&mut size, 11.0..=22.0).step_by(1.0).suffix(" px")).changed() {
                                    ctx.settings.editor_font_size = size;
                                    let _ = ctx.settings.save_to_file(settings_path);
                                }
                            });
                        });

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Font Family").color(t.text).size(13.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let fonts = ["System Default", "DejaVu Sans", "Noto Sans", "Unifont"];
                                egui::ComboBox::from_id_salt("settings_font_picker")
                                    .selected_text(&ctx.settings.font_family)
                                    .show_ui(ui, |ui| {
                                        for f in fonts {
                                            if ui.selectable_label(ctx.settings.font_family == f, f).clicked() {
                                                ctx.settings.font_family = f.to_owned();
                                                let _ = ctx.settings.save_to_file(settings_path);
                                            }
                                        }
                                    });
                            });
                        });
                    });

                    ui.add_space(12.0);

                    // Section 3: Favorite Color Palette
                    theme::section_label(ui, &t, "Favorite Colors Palette");
                    ui.add_space(4.0);

                    let palette_card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(14, 10));
                    palette_card.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Colors available across Quick Notes, Sidebar & Emojis").color(t.text_dim).size(12.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if theme::ghost_button(ui, &t, RichText::new("Reset Defaults").size(11.5).color(t.text_dim)).clicked() {
                                    ctx.settings.color_palette = default_color_palette();
                                    let _ = ctx.settings.save_to_file(settings_path);
                                }
                            });
                        });

                        ui.add_space(8.0);

                        let mut to_remove = None;
                        for (idx, color_def) in ctx.settings.color_palette.iter().enumerate() {
                            let col = theme::parse_hex_color(&color_def.hex).unwrap_or(t.text_dim);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                                ui.painter().circle_filled(rect.center(), 7.0, col);
                                ui.painter().circle_stroke(rect.center(), 8.0, Stroke::new(1.0, t.field_border));

                                ui.add_space(4.0);
                                ui.label(RichText::new(&color_def.label).size(12.5).color(t.text));
                                ui.label(RichText::new(&color_def.hex).size(11.0).monospace().color(t.text_faint));

                                if color_def.name != "default" {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if theme::ghost_button(ui, &t, RichText::new("×").size(13.0).color(t.text_faint)).clicked() {
                                            to_remove = Some(idx);
                                        }
                                    });
                                }
                            });
                            ui.add_space(2.0);
                        }

                        if let Some(idx) = to_remove {
                            ctx.settings.color_palette.remove(idx);
                            let _ = ctx.settings.save_to_file(settings_path);
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.label(RichText::new("Add Color to Favorites:").size(12.0).color(t.text_dim));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let preview_col = theme::parse_hex_color(&modal_state.new_color_hex).unwrap_or(t.accent);
                            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                            ui.painter().circle_filled(rect.center(), 8.0, preview_col);
                            ui.painter().circle_stroke(rect.center(), 9.0, Stroke::new(1.0, t.field_border));

                            ui.add_space(2.0);
                            ui.add(egui::TextEdit::singleline(&mut modal_state.new_color_hex).hint_text("#RRGGBB").desired_width(75.0));
                            ui.add(egui::TextEdit::singleline(&mut modal_state.new_color_label).hint_text("Label (e.g. Amber)").desired_width(120.0));

                            let can_add = theme::parse_hex_color(&modal_state.new_color_hex).is_some();
                            if ui.add_enabled(
                                can_add,
                                egui::Button::new(RichText::new("+ Add").size(11.5).color(Color32::WHITE))
                                    .fill(t.accent)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                            ).clicked() {
                                let hex = if modal_state.new_color_hex.starts_with('#') {
                                    modal_state.new_color_hex.trim().to_string()
                                } else {
                                    format!("#{}", modal_state.new_color_hex.trim())
                                };
                                let label = if modal_state.new_color_label.trim().is_empty() {
                                    hex.clone()
                                } else {
                                    modal_state.new_color_label.trim().to_string()
                                };
                                let name = label.to_lowercase().replace(' ', "-");
                                ctx.settings.color_palette.push(ColorDef { name, label, hex });
                                modal_state.new_color_label.clear();
                                let _ = ctx.settings.save_to_file(settings_path);
                            }
                        });
                    });

                    ui.add_space(12.0);

                    // Section 4: Vault & Folders
                    theme::section_label(ui, &t, "Vault & Folders");
                    ui.add_space(4.0);

                    let card = theme::card_frame(&t, false).inner_margin(Margin::symmetric(14, 10));
                    card.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Daily Notes Folder").color(t.text).size(13.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::TextEdit::singleline(&mut ctx.settings.daily_folder).desired_width(140.0)).changed() {
                                    let _ = ctx.settings.save_to_file(settings_path);
                                }
                            });
                        });

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Events Folder").color(t.text).size(13.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::TextEdit::singleline(&mut ctx.settings.events_folder).desired_width(140.0)).changed() {
                                    let _ = ctx.settings.save_to_file(settings_path);
                                }
                            });
                        });
                    });
                }

                SettingsTab::Plugins => {
                    // DEDICATED PLUGINS SECTION
                    theme::section_label(ui, &t, "Installed Feature Plugins");
                    ui.add_space(2.0);
                    ui.label(RichText::new("Dynamically enable or disable built-in features at runtime without restarting.").color(t.text_dim).size(12.0));
                    ui.add_space(8.0);

                    let plugins = registry.plugins_info();
                    for info in plugins {
                        let card = theme::card_frame(&t, info.enabled).inner_margin(Margin::same(12));
                        card.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(info.name).strong().size(14.0).color(t.text));
                                        let type_str = match info.view_type {
                                            PluginViewType::MainWorkspace => "Main Workspace",
                                            PluginViewType::SidePanel => "Side Panel",
                                            PluginViewType::RightPanel => "Right Panel",
                                            PluginViewType::ModalOverlay => "Modal Dialog",
                                            PluginViewType::Background => "Background",
                                        };
                                        theme::pill(ui, type_str, t.text_faint, t.field);
                                        ui.label(RichText::new(format!("({})", info.id)).color(t.text_faint).size(10.5));
                                    });
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(info.description).color(t.text_dim).size(12.0));
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let btn_text = if info.enabled { "Enabled ●" } else { "Disabled ○" };
                                    let btn_color = if info.enabled { t.success } else { t.text_faint };
                                    let btn_bg = if info.enabled { t.success.gamma_multiply(0.15) } else { t.field };

                                    let toggle_btn = egui::Button::new(
                                        RichText::new(btn_text).color(btn_color).strong().size(12.0),
                                    )
                                    .fill(btn_bg)
                                    .stroke(Stroke::new(1.0, if info.enabled { t.success } else { t.field_border }))
                                    .corner_radius(CornerRadius::same(t.radius_sm as u8));

                                    if ui.add(toggle_btn).clicked() {
                                        plugin_toggle_req = Some((info.id.to_string(), !info.enabled));
                                    }
                                });
                            });
                        });
                        ui.add_space(6.0);
                    }
                }
            }
        });
    });

    if let Some(kind) = theme_changed {
        *ctx.theme_kind = kind;
        ctx.settings.theme = kind.to_string();
        theme::apply_theme(ctx_egui, kind);
        let _ = ctx.settings.save_to_file(settings_path);
    }

    if let Some((id, new_state)) = plugin_toggle_req {
        if new_state {
            registry.enable_plugin(&id, ctx);
            ctx.toast(format!("Enabled plugin: {id}"));
        } else {
            registry.disable_plugin(&id, ctx);
            // If the disabled plugin was currently active, clear or fall back!
            if active_main.as_deref() == Some(&id) {
                *active_main = None;
            }
            if active_side.as_deref() == Some(&id) {
                *active_side = None;
            }
            if active_right.as_deref() == Some(&id) {
                *active_right = None;
            }
            ctx.toast(format!("Disabled plugin: {id}"));
        }
        let _ = ctx.settings.save_to_file(settings_path);
    }

    if modal_resp.should_close() || should_close {
        *is_open = false;
    }
}
