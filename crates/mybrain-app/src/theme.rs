//! Obsidian-inspired sleek dark theme and UI helpers for MyBrain.

use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BG_CANVAS: Color32 = Color32::from_rgb(20, 20, 24);
pub const BG_PANEL: Color32 = Color32::from_rgb(26, 26, 32);
pub const BG_CARD: Color32 = Color32::from_rgb(34, 34, 42);
pub const BG_HOVER: Color32 = Color32::from_rgb(44, 44, 56);
pub const BG_ACTIVE: Color32 = Color32::from_rgb(52, 52, 66);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(225, 225, 235);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(135, 135, 150);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(90, 90, 105);

pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(142, 107, 242);
pub const ACCENT_PURPLE_LIGHT: Color32 = Color32::from_rgb(175, 145, 255);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(85, 205, 195);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(240, 180, 80);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(110, 200, 130);
pub const ACCENT_RED: Color32 = Color32::from_rgb(235, 95, 95);

pub const BORDER_COLOR: Color32 = Color32::from_rgb(46, 46, 58);

/// Applies custom Obsidian dark theme visuals to an egui context.
pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_CARD;
    visuals.extreme_bg_color = BG_CANVAS;

    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_COLOR);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = BG_CARD;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_COLOR);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_PURPLE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = BG_ACTIVE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5, ACCENT_PURPLE_LIGHT);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(142, 107, 242, 90);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_PURPLE_LIGHT);

    ctx.set_visuals(visuals);
}
