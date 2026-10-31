//! Permanent vertical left icon ribbon bar.

use egui::{vec2, Align2, Color32, CornerRadius, CursorIcon, Frame, Margin, Rect, Sense, Stroke, Ui};

use crate::plugins::{PluginRegistry, PluginViewType, RibbonSlot};
use crate::theme::{self, Tokens};

pub struct RibbonAction {
    pub toggle_side: Option<String>,
    pub toggle_right: Option<String>,
    pub switch_main: Option<String>,
    pub open_modal: Option<String>,
    pub open_command_palette: bool,
    pub open_vault_switcher: bool,
    pub open_settings: bool,
}

impl Default for RibbonAction {
    fn default() -> Self {
        Self {
            toggle_side: None,
            toggle_right: None,
            switch_main: None,
            open_modal: None,
            open_command_palette: false,
            open_vault_switcher: false,
            open_settings: false,
        }
    }
}

/// Render the permanent, non-collapsible vertical ribbon on the left edge of the window.
pub fn render_ribbon(
    ui: &mut Ui,
    registry: &PluginRegistry,
    active_main: Option<&str>,
    active_side: Option<&str>,
    active_right: Option<&str>,
    t: &Tokens,
) -> RibbonAction {
    let mut action = RibbonAction::default();

    let ribbon_frame = Frame::NONE
        .fill(t.dock)
        .stroke(Stroke::new(1.0, t.separator))
        .inner_margin(Margin::symmetric(4, 8));

    ribbon_frame.show(ui, |ui| {
        ui.set_width(42.0);
        let _max_height = ui.available_height();

        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;

            // Top Stack: Dynamic plugin buttons
            let buttons = registry.enabled_ribbon_buttons();
            let top_buttons: Vec<_> = buttons.into_iter().filter(|(_, b)| b.slot == RibbonSlot::Top).collect();

            for (id, btn) in top_buttons {
                let is_active = match btn.view_type {
                    PluginViewType::MainWorkspace => active_main == Some(id),
                    PluginViewType::SidePanel => active_side == Some(id),
                    PluginViewType::RightPanel => active_right == Some(id),
                    _ => false,
                };

                let resp = render_ribbon_icon_button(ui, btn.icon, btn.tooltip, is_active, t);
                if resp.clicked() {
                    match btn.view_type {
                        PluginViewType::SidePanel => {
                            action.toggle_side = Some(id.to_string());
                        }
                        PluginViewType::RightPanel => {
                            action.toggle_right = Some(id.to_string());
                        }
                        PluginViewType::MainWorkspace => {
                            action.switch_main = Some(id.to_string());
                        }
                        PluginViewType::ModalOverlay => {
                            action.open_modal = Some(id.to_string());
                        }
                        PluginViewType::Background => {}
                    }
                }
            }

            // Push bottom pinned actions to bottom of window
            let remaining = ui.available_height() - 120.0;
            if remaining > 0.0 {
                ui.add_space(remaining);
            }

            ui.separator();

            // Bottom Stack: Pinned Shell Actions
            // 1. Command Palette
            let cp_resp = render_ribbon_icon_button(ui, "🔍", "Command Palette (Ctrl+P)", false, t);
            if cp_resp.clicked() {
                action.open_command_palette = true;
            }

            // 2. Vault Switcher
            let vault_resp = render_ribbon_icon_button(ui, "🗄️", "Switch Vault (Ctrl+O)", false, t);
            if vault_resp.clicked() {
                action.open_vault_switcher = true;
            }

            // 3. Settings / Plugin Manager
            let settings_resp = render_ribbon_icon_button(ui, "⚙️", "Settings & Plugins (Ctrl+,)", false, t);
            if settings_resp.clicked() {
                action.open_settings = true;
            }
        });
    });

    action
}

fn render_ribbon_icon_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    is_active: bool,
    t: &Tokens,
) -> egui::Response {
    let btn_size = vec2(36.0, 36.0);
    let (rect, resp) = ui.allocate_exact_size(btn_size, Sense::click());
    let hovered = resp.hovered();

    let bg = if is_active {
        t.accent.gamma_multiply(0.2)
    } else if hovered {
        t.hover
    } else {
        Color32::TRANSPARENT
    };

    ui.painter().rect_filled(rect, CornerRadius::same(t.radius_sm as u8), bg);

    if is_active {
        // Active indicator line on the left edge
        let indicator = Rect::from_min_size(rect.left_top() + vec2(-2.0, 6.0), vec2(3.0, 24.0));
        ui.painter().rect_filled(indicator, CornerRadius::same(2), t.accent);
    }

    let color = if is_active {
        t.accent_text
    } else if hovered {
        t.text
    } else {
        t.text_dim
    };

    if icon == "calendar" || icon == "📅" || icon == "🗓" {
        theme::paint_calendar_icon(ui.painter(), rect, color);
    } else if icon == "graph" || icon == "🕸️" {
        theme::paint_graph_icon(ui.painter(), rect, color);
    } else {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(16.0),
            color,
        );
    }

    resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tooltip)
}
