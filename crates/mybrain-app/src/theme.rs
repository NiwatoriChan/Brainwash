//! PhotoCraft-inspired sleek design system and theme tokens for MyBrain.
//!
//! Based on https://github.com/storytold/photocraft design system:
//! - **Studio** (Dark): Near-black surfaces, soft violet accent, rounded cards, thin scrollbars.
//! - **Studio Light**: Clean, low-glare light surfaces with purple accent.
//! - **Pro**: Photoshop-inspired dark charcoal surfaces with electric blue accent.
//! - **Solarized Dark**: Classic Ethan Schoonover palette (base03/base02/base0 with Solarized blue).
//! - **Adwaita Dark**: GNOME libadwaita palette with blue accents.

use std::sync::Arc;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    /// PhotoCraft Studio: near-black surfaces, rounded cards, soft violet accent.
    #[default]
    Studio,
    /// PhotoCraft Studio Light: light surfaces, purple accent.
    StudioLight,
    /// Photoshop Spectrum Pro dark: flat charcoal panels, blue accent.
    Pro,
    /// Ethan Schoonover's Solarized Dark palette.
    SolarizedDark,
    /// GNOME libadwaita dark theme.
    AdwaitaDark,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 5] = [
        ThemeKind::Studio,
        ThemeKind::StudioLight,
        ThemeKind::Pro,
        ThemeKind::SolarizedDark,
        ThemeKind::AdwaitaDark,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::Studio => "Studio (Dark)",
            ThemeKind::StudioLight => "Studio (Light)",
            ThemeKind::Pro => "Pro (Charcoal)",
            ThemeKind::SolarizedDark => "Solarized Dark",
            ThemeKind::AdwaitaDark => "Adwaita Dark",
        }
    }

    pub fn is_dark(self) -> bool {
        !matches!(self, ThemeKind::StudioLight)
    }

    #[allow(dead_code)]
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// Colour and shape tokens adapted directly from PhotoCraft.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub kind: ThemeKind,
    pub chrome: Color32,
    pub canvas: Color32,
    pub dock: Color32,
    pub card: Color32,
    pub card_border: Color32,
    pub field: Color32,
    pub field_border: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub icon: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub accent_border: Color32,
    pub accent_text: Color32,
    pub separator: Color32,
    pub shadow: Color32,
    pub primary_bg: Color32,
    pub primary_text: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub success: Color32,
    pub radius_sm: f32,
    pub radius: f32,
    pub radius_lg: f32,
}

impl Tokens {
    pub fn for_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Studio => Tokens {
                kind,
                chrome: Color32::from_rgb(20, 20, 21),
                canvas: Color32::from_rgb(14, 14, 15),
                dock: Color32::from_rgb(17, 17, 18),
                card: Color32::from_rgb(26, 26, 28),
                card_border: Color32::from_rgb(40, 40, 44),
                field: Color32::from_rgb(35, 35, 38),
                field_border: Color32::from_rgb(52, 52, 57),
                hover: Color32::from_rgb(44, 44, 48),
                pressed: Color32::from_rgb(56, 56, 62),
                text: Color32::from_rgb(236, 236, 240),
                text_dim: Color32::from_rgb(150, 150, 158),
                text_faint: Color32::from_rgb(96, 96, 104),
                icon: Color32::from_rgb(196, 196, 204),
                accent: Color32::from_rgb(139, 124, 246),
                accent_soft: Color32::from_rgba_unmultiplied(139, 124, 246, 46),
                accent_border: Color32::from_rgba_unmultiplied(160, 148, 255, 110),
                accent_text: Color32::from_rgb(214, 208, 255),
                separator: Color32::from_rgb(38, 38, 42),
                shadow: Color32::from_black_alpha(140),
                primary_bg: Color32::from_rgb(246, 246, 248),
                primary_text: Color32::from_rgb(12, 12, 14),
                danger: Color32::from_rgb(240, 96, 96),
                warning: Color32::from_rgb(240, 190, 90),
                success: Color32::from_rgb(105, 210, 135),
                radius_sm: 6.0,
                radius: 8.0,
                radius_lg: 12.0,
            },
            ThemeKind::StudioLight => Tokens {
                kind,
                chrome: Color32::from_rgb(246, 246, 248),
                canvas: Color32::from_rgb(226, 226, 230),
                dock: Color32::from_rgb(240, 240, 243),
                card: Color32::from_rgb(252, 252, 253),
                card_border: Color32::from_rgb(222, 222, 228),
                field: Color32::from_rgb(242, 242, 245),
                field_border: Color32::from_rgb(214, 214, 220),
                hover: Color32::from_rgb(232, 232, 237),
                pressed: Color32::from_rgb(220, 220, 226),
                text: Color32::from_rgb(24, 24, 28),
                text_dim: Color32::from_rgb(96, 96, 106),
                text_faint: Color32::from_rgb(150, 150, 160),
                icon: Color32::from_rgb(60, 60, 68),
                accent: Color32::from_rgb(108, 92, 231),
                accent_soft: Color32::from_rgba_unmultiplied(108, 92, 231, 36),
                accent_border: Color32::from_rgba_unmultiplied(108, 92, 231, 120),
                accent_text: Color32::from_rgb(80, 64, 200),
                separator: Color32::from_rgb(226, 226, 232),
                shadow: Color32::from_black_alpha(50),
                primary_bg: Color32::from_rgb(20, 20, 24),
                primary_text: Color32::from_rgb(250, 250, 252),
                danger: Color32::from_rgb(210, 60, 60),
                warning: Color32::from_rgb(190, 130, 20),
                success: Color32::from_rgb(50, 160, 80),
                radius_sm: 6.0,
                radius: 8.0,
                radius_lg: 12.0,
            },
            ThemeKind::Pro => Tokens {
                kind,
                chrome: Color32::from_rgb(50, 50, 50),
                canvas: Color32::from_rgb(40, 40, 40),
                dock: Color32::from_rgb(30, 30, 30),
                card: Color32::from_rgb(50, 50, 50),
                card_border: Color32::from_rgb(30, 30, 30),
                field: Color32::from_rgb(36, 36, 36),
                field_border: Color32::from_rgb(74, 74, 74),
                hover: Color32::from_rgb(66, 66, 66),
                pressed: Color32::from_rgb(78, 78, 78),
                text: Color32::from_rgb(222, 222, 222),
                text_dim: Color32::from_rgb(178, 178, 178),
                text_faint: Color32::from_rgb(128, 128, 128),
                icon: Color32::from_rgb(200, 200, 200),
                accent: Color32::from_rgb(55, 142, 240),
                accent_soft: Color32::from_rgb(78, 78, 78),
                accent_border: Color32::from_rgb(55, 142, 240),
                accent_text: Color32::WHITE,
                separator: Color32::from_rgb(30, 30, 30),
                shadow: Color32::from_black_alpha(150),
                primary_bg: Color32::from_rgb(55, 142, 240),
                primary_text: Color32::WHITE,
                danger: Color32::from_rgb(236, 91, 98),
                warning: Color32::from_rgb(232, 176, 70),
                success: Color32::from_rgb(90, 200, 120),
                radius_sm: 4.0,
                radius: 6.0,
                radius_lg: 8.0,
            },
            ThemeKind::SolarizedDark => Tokens {
                kind,
                chrome: Color32::from_rgb(7, 54, 66),
                canvas: Color32::from_rgb(0, 43, 54),
                dock: Color32::from_rgb(0, 37, 47),
                card: Color32::from_rgb(7, 54, 66),
                card_border: Color32::from_rgb(0, 37, 47),
                field: Color32::from_rgb(0, 37, 47),
                field_border: Color32::from_rgb(21, 80, 95),
                hover: Color32::from_rgb(14, 74, 90),
                pressed: Color32::from_rgb(18, 89, 107),
                text: Color32::from_rgb(131, 148, 150),
                text_dim: Color32::from_rgb(101, 123, 131),
                text_faint: Color32::from_rgb(88, 110, 117),
                icon: Color32::from_rgb(147, 161, 161),
                accent: Color32::from_rgb(38, 139, 210),
                accent_soft: Color32::from_rgba_unmultiplied(38, 139, 210, 46),
                accent_border: Color32::from_rgba_unmultiplied(38, 139, 210, 120),
                accent_text: Color32::from_rgb(253, 246, 227),
                separator: Color32::from_rgb(0, 37, 47),
                shadow: Color32::from_black_alpha(150),
                primary_bg: Color32::from_rgb(38, 139, 210),
                primary_text: Color32::from_rgb(253, 246, 227),
                danger: Color32::from_rgb(220, 50, 47),
                warning: Color32::from_rgb(181, 137, 0),
                success: Color32::from_rgb(133, 153, 0),
                radius_sm: 6.0,
                radius: 8.0,
                radius_lg: 12.0,
            },
            ThemeKind::AdwaitaDark => Tokens {
                kind,
                chrome: Color32::from_rgb(46, 46, 50),
                canvas: Color32::from_rgb(29, 29, 32),
                dock: Color32::from_rgb(34, 34, 38),
                card: Color32::from_rgb(54, 54, 58),
                card_border: Color32::from_rgb(64, 64, 68),
                field: Color32::from_rgb(64, 64, 68),
                field_border: Color32::from_rgb(76, 76, 80),
                hover: Color32::from_rgb(72, 72, 76),
                pressed: Color32::from_rgb(86, 86, 90),
                text: Color32::WHITE,
                text_dim: Color32::from_rgb(192, 192, 194),
                text_faint: Color32::from_rgb(145, 145, 148),
                icon: Color32::from_rgb(235, 235, 237),
                accent: Color32::from_rgb(53, 132, 228),
                accent_soft: Color32::from_rgba_unmultiplied(53, 132, 228, 60),
                accent_border: Color32::from_rgba_unmultiplied(120, 174, 237, 140),
                accent_text: Color32::from_rgb(120, 174, 237),
                separator: Color32::from_rgb(24, 24, 27),
                shadow: Color32::from_black_alpha(140),
                primary_bg: Color32::from_rgb(53, 132, 228),
                primary_text: Color32::WHITE,
                danger: Color32::from_rgb(255, 123, 99),
                warning: Color32::from_rgb(248, 228, 92),
                success: Color32::from_rgb(87, 227, 137),
                radius_sm: 6.0,
                radius: 9.0,
                radius_lg: 12.0,
            },
        }
    }

    /// Retrieve active tokens stored in egui context by [`apply_theme`].
    pub fn get(ctx: &egui::Context) -> Tokens {
        ctx.data(|d| d.get_temp::<Tokens>(egui::Id::NULL.with("mybrain-theme")))
            .unwrap_or_else(|| Tokens::for_kind(ThemeKind::Studio))
    }
}

// Global convenience color constants matching PhotoCraft Studio theme
#[allow(dead_code)]
pub const BG_CANVAS: Color32 = Color32::from_rgb(14, 14, 15);
#[allow(dead_code)]
pub const BG_PANEL: Color32 = Color32::from_rgb(20, 20, 21);
#[allow(dead_code)]
pub const BG_CARD: Color32 = Color32::from_rgb(26, 26, 28);
#[allow(dead_code)]
pub const BG_HOVER: Color32 = Color32::from_rgb(44, 44, 48);
#[allow(dead_code)]
pub const BG_ACTIVE: Color32 = Color32::from_rgb(56, 56, 62);

#[allow(dead_code)]
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(236, 236, 240);
#[allow(dead_code)]
pub const TEXT_MUTED: Color32 = Color32::from_rgb(150, 150, 158);
#[allow(dead_code)]
pub const TEXT_FAINT: Color32 = Color32::from_rgb(96, 96, 104);

#[allow(dead_code)]
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(139, 124, 246);
#[allow(dead_code)]
pub const ACCENT_PURPLE_LIGHT: Color32 = Color32::from_rgb(214, 208, 255);
#[allow(dead_code)]
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(85, 205, 195);
#[allow(dead_code)]
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(240, 190, 90);
#[allow(dead_code)]
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(105, 210, 135);
#[allow(dead_code)]
pub const ACCENT_RED: Color32 = Color32::from_rgb(240, 96, 96);

pub const BORDER_COLOR: Color32 = Color32::from_rgb(40, 40, 44);

/// Install font fallback stack to prevent missing glyph tofu.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Check system fonts for extra fallback symbols/glyphs
    let candidate_fonts = [
        "/nix/store/nb12xsc3aj49hgr0jqjqp9gnd6l9lxj9-nerd-fonts-hack-3.4.0+3.003/share/fonts/truetype/NerdFonts/Hack/HackNerdFont-Regular.ttf",
        "/nix/store/349bajb6xdfd4mw3in7bs8jkpmhx1w96-dejavu-fonts-2.37/share/fonts/truetype/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    for path in candidate_fonts {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "system_symbol_fallback".to_owned(),
                Arc::new(FontData::from_owned(bytes)),
            );
            if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
                prop.push("system_symbol_fallback".to_owned());
            }
            if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
                mono.push("system_symbol_fallback".to_owned());
            }
            break;
        }
    }

    ctx.set_fonts(fonts);
}

/// Applies PhotoCraft theme tokens and visuals to an egui context.
pub fn apply_theme(ctx: &egui::Context, kind: ThemeKind) {
    ctx.set_theme(egui::ThemePreference::System);
    let t = Tokens::for_kind(kind);
    ctx.data_mut(|d| d.insert_temp(egui::Id::NULL.with("mybrain-theme"), t));

    let mut v = if kind.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    v.panel_fill = t.chrome;
    v.window_fill = t.card;
    v.window_stroke = Stroke::new(1.0, t.card_border);
    v.extreme_bg_color = t.field;
    v.faint_bg_color = t.card;
    v.code_bg_color = t.field;
    v.override_text_color = Some(t.text);
    v.hyperlink_color = t.accent;
    v.warn_fg_color = t.warning;
    v.error_fg_color = t.danger;
    v.window_corner_radius = CornerRadius::same(t.radius_lg as u8);
    v.menu_corner_radius = CornerRadius::same(t.radius as u8);
    v.selection.bg_fill = t.accent_soft;
    v.selection.stroke = Stroke::new(1.0, t.accent_text);
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.striped = false;

    let r = CornerRadius::same(t.radius_sm as u8);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = t.card;
    w.noninteractive.weak_bg_fill = t.card;
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.separator);
    w.noninteractive.fg_stroke = Stroke::new(1.0, t.text_dim);
    w.noninteractive.corner_radius = r;

    for (wv, bg, stroke) in [
        (&mut w.inactive, t.field, t.field_border),
        (&mut w.hovered, t.hover, t.field_border),
        (&mut w.active, t.pressed, t.accent_border),
        (&mut w.open, t.hover, t.field_border),
    ] {
        wv.bg_fill = bg;
        wv.weak_bg_fill = bg;
        wv.bg_stroke = Stroke::new(1.0, stroke);
        wv.fg_stroke = Stroke::new(1.0, t.text);
        wv.corner_radius = r;
        wv.expansion = 0.0;
    }

    ctx.set_visuals(v);

    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size = egui::vec2(24.0, 24.0);
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.window_margin = egui::Margin::same(16);
        s.spacing.icon_width = 14.0;
        s.visuals.indent_has_left_vline = false;

        // Thin PhotoCraft / macOS floating scrollbars
        let mut scroll = egui::style::ScrollStyle::thin();
        if scroll.floating {
            scroll.interact_handle_opacity = 1.0;
            scroll.interact_background_opacity = 0.4;
            scroll.floating_width = 5.0;
        }
        s.spacing.scroll = scroll;
    });

    let style = ctx.global_style();
    ctx.set_style_of(egui::Theme::Light, Arc::clone(&style));
    ctx.set_style_of(egui::Theme::Dark, style);
}
