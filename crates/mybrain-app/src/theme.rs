//! PhotoCraft-inspired sleek design system and theme tokens for MyBrain.
//!
//! Based on https://github.com/storytold/photocraft design system:
//! - **Studio** (Dark): Near-black surfaces, soft violet accent, rounded cards, thin scrollbars.
//! - **Studio Light**: Clean, low-glare light surfaces with purple accent.
//! - **Pro**: Photoshop-inspired dark charcoal surfaces with electric blue accent.
//! - **Solarized Dark**: Classic Ethan Schoonover palette (base03/base02/base0 with Solarized blue).
//! - **Adwaita Dark**: GNOME libadwaita palette with blue accents.

use std::sync::Arc;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily, FontId,
    Frame, Margin, Rect, Response, RichText, Sense, Stroke, TextFormat, Ui, Visuals,
};
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

// ---------------------------------------------------------------------------
// Shared themed widgets
// ---------------------------------------------------------------------------

/// Maximum width of rendered prose / editor text for comfortable reading.
pub const READABLE_WIDTH: f32 = 760.0;

/// Kind of note, derived from its folder, used for coloured dots in lists.
pub fn note_kind_color(id: &str, t: &Tokens) -> Color32 {
    if id.starts_with("Daily/") {
        t.accent
    } else if id.starts_with("Events/") {
        t.warning
    } else if id.starts_with("Quick Notes/") {
        t.success
    } else {
        t.text_faint
    }
}

pub fn truncated_galley(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
) -> Arc<egui::Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(font, color));
    job.wrap = TextWrapping::truncate_at_width(max_width.max(0.0));
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

/// Small uppercase section caption (e.g. "EXPLORER", "BACKLINKS").
pub fn section_label(ui: &mut Ui, t: &Tokens, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(10.5)
            .strong()
            .color(t.text_faint),
    );
}

/// Checks if a string begins with an emoji, returning the emoji substring and the remainder of the text.
pub fn split_leading_emoji(s: &str) -> Option<(&str, &str)> {
    let mut chars = s.chars().peekable();
    let first = *chars.peek()?;
    let is_emoji_char = |c: char| {
        (c >= '\u{1F000}' && c <= '\u{1FAFF}')
            || (c >= '\u{2600}' && c <= '\u{27BF}')
            || (c >= '\u{2300}' && c <= '\u{23FF}')
            || (c >= '\u{2B50}' && c <= '\u{2B55}')
            || c == '\u{FE0E}'
            || c == '\u{FE0F}'
            || c == '\u{200D}'
    };

    if !is_emoji_char(first) || first == '\u{FE0E}' || first == '\u{FE0F}' || first == '\u{200D}' {
        return None;
    }

    let mut byte_len = 0;
    while let Some(&c) = chars.peek() {
        if is_emoji_char(c) {
            byte_len += c.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    if byte_len == 0 {
        return None;
    }

    let emoji = &s[..byte_len];
    let rest = s[byte_len..].trim_start();
    Some((emoji, rest))
}

/// Full-width, single-line clickable list row with hover / selected states.
///
/// * `dot`: optional coloured bullet on the left. If `title` begins with an emoji,
///   the emoji itself is rendered directly in `dot`'s colour, merging the icon and colour.
/// * `trailing`: optional faint text right-aligned (e.g. folder name).
pub fn list_row(
    ui: &mut Ui,
    t: &Tokens,
    selected: bool,
    dot: Option<Color32>,
    title: &str,
    trailing: Option<&str>,
) -> Response {
    let height = 26.0;
    let (rect, resp) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());

    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(t.radius_sm as u8);
        let painter = ui.painter();

        if selected {
            painter.rect_filled(rect, radius, t.accent_soft);
            let bar = Rect::from_min_size(
                pos2(rect.left(), rect.top() + 5.0),
                vec2(3.0, rect.height() - 10.0),
            );
            painter.rect_filled(bar, CornerRadius::same(2), t.accent);
        } else if resp.hovered() {
            painter.rect_filled(rect, radius, t.hover);
        }

        let mut x = rect.left() + 10.0;
        let mut right = rect.right() - 8.0;
        if let Some(trail) = trailing {
            let g = truncated_galley(ui, trail, FontId::proportional(10.5), t.text_faint, 90.0);
            right -= g.size().x;
            painter.galley(pos2(right, rect.center().y - g.size().y / 2.0), g, t.text_faint);
            right -= 8.0;
        }

        let color = if selected { t.text } else if resp.hovered() { t.text } else { t.text_dim };

        if let Some((emoji, rest)) = split_leading_emoji(title) {
            // Merge color and emoji: tint emoji with chosen dot color directly, omitting redundant circle dot.
            let emoji_color = dot.unwrap_or(color);
            let mut job = LayoutJob::default();
            job.append(
                emoji,
                0.0,
                TextFormat::simple(FontId::proportional(13.0), emoji_color),
            );
            job.append(
                " ",
                0.0,
                TextFormat::simple(FontId::proportional(13.0), color),
            );
            job.append(
                rest,
                0.0,
                TextFormat::simple(FontId::proportional(13.0), color),
            );
            job.wrap = TextWrapping::truncate_at_width((right - x).max(0.0));
            let g = ui.ctx().fonts_mut(|f| f.layout_job(job));
            painter.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, color);
        } else {
            if let Some(c) = dot {
                painter.circle_filled(pos2(x + 3.0, rect.center().y), 3.0, c);
                x += 14.0;
            }
            let g = truncated_galley(ui, title, FontId::proportional(13.0), color, right - x);
            painter.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, color);
        }
    }

    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Parse hex code (e.g. "#RRGGBB" or "#RGB") into a Color32.
pub fn parse_hex_color(hex_str: &str) -> Option<Color32> {
    let hex = hex_str.trim().strip_prefix('#').unwrap_or(hex_str.trim());
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(Color32::from_rgb(r, g, b));
    } else if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
        return Some(Color32::from_rgb(r, g, b));
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorPickerAction {
    SetColor(Option<String>),
    OpenCustomColor,
}

/// Renders the unified color selector row of circular color dots using the customizable palette,
/// plus a round "+" button at the end for custom colors.
pub fn color_picker_dots(
    ui: &mut Ui,
    t: &Tokens,
    palette: &[mybrain_core::settings::ColorDef],
    current_color: Option<&str>,
) -> Option<ColorPickerAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for color_def in palette {
            let dot_col = parse_hex_color(&color_def.hex).unwrap_or(t.text_dim);
            let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
            let is_curr = current_color
                .map(|c| c.eq_ignore_ascii_case(&color_def.name) || c.eq_ignore_ascii_case(&color_def.hex))
                .unwrap_or(color_def.name == "default");

            ui.painter().circle_filled(
                rect.center(),
                7.0,
                dot_col,
            );
            if is_curr {
                ui.painter().circle_stroke(
                    rect.center(),
                    8.5,
                    Stroke::new(1.5, t.text),
                );
            }
            if resp
                .on_hover_text(&color_def.label)
                .on_hover_cursor(CursorIcon::PointingHand)
                .clicked()
            {
                if color_def.name == "default" {
                    action = Some(ColorPickerAction::SetColor(None));
                } else {
                    action = Some(ColorPickerAction::SetColor(Some(color_def.name.clone())));
                }
            }
        }

        // Custom Color round "+" button at the end of the line
        let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
        let is_custom_selected = current_color
            .map(|c| !c.eq_ignore_ascii_case("default") && !palette.iter().any(|d| d.name.eq_ignore_ascii_case(c) || d.hex.eq_ignore_ascii_case(c)))
            .unwrap_or(false);

        let custom_color32 = if is_custom_selected {
            current_color.and_then(crate::state::parse_color_str)
        } else {
            None
        };

        if let Some(col) = custom_color32 {
            ui.painter().circle_filled(rect.center(), 7.0, col);
            ui.painter().circle_stroke(
                rect.center(),
                8.5,
                Stroke::new(1.5, t.text),
            );
        } else {
            ui.painter().circle_stroke(
                rect.center(),
                7.0,
                Stroke::new(1.0, t.separator),
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "+",
                egui::FontId::proportional(12.0),
                t.text_dim,
            );
        }

        if resp
            .on_hover_text("Custom Color")
            .on_hover_cursor(CursorIcon::PointingHand)
            .clicked()
        {
            action = Some(ColorPickerAction::OpenCustomColor);
        }
    });
    action
}

/// Segmented control (pill group). Returns the option clicked this frame, if any.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    t: &Tokens,
    current: T,
    options: &[(T, &str)],
) -> Option<T> {
    let mut clicked = None;
    Frame::NONE
        .fill(t.field)
        .stroke(Stroke::new(1.0, t.separator))
        .corner_radius(CornerRadius::same(t.radius as u8))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for (value, label) in options {
                    let selected = *value == current;
                    let font = FontId::proportional(12.5);
                    let g = truncated_galley(ui, label, font, t.text, f32::INFINITY);
                    let size = vec2(g.size().x + 20.0, 24.0);
                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                    let radius = CornerRadius::same((t.radius as u8).saturating_sub(2));
                    let color = if selected {
                        ui.painter().rect_filled(rect, radius, t.card);
                        ui.painter().rect_stroke(
                            rect,
                            radius,
                            Stroke::new(1.0, t.card_border),
                            egui::StrokeKind::Inside,
                        );
                        t.text
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, radius, t.hover);
                        t.text
                    } else {
                        t.text_dim
                    };
                    ui.painter().galley(rect.center() - g.size() / 2.0, g, color);
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        clicked = Some(*value);
                    }
                }
            });
        });
    clicked
}

/// Frameless button that only shows a background when hovered.
pub fn ghost_button(ui: &mut Ui, t: &Tokens, text: impl Into<RichText>) -> Response {
    let rt: RichText = text.into();
    ui.add(
        egui::Button::new(rt)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
}

/// Filled accent call-to-action button.
#[allow(dead_code)]
pub fn primary_button(ui: &mut Ui, t: &Tokens, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).strong().color(Color32::WHITE))
            .fill(t.accent)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
}

/// Rounded tag/badge pill.
pub fn pill(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    Frame::NONE
        .fill(bg)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(fg).size(11.0));
        });
}

/// Standard card frame used for list items and panels.
pub fn card_frame(t: &Tokens, highlighted: bool) -> Frame {
    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(
            1.0,
            if highlighted { t.accent_border } else { t.card_border },
        ))
        .corner_radius(CornerRadius::same(t.radius as u8))
        .inner_margin(Margin::symmetric(12, 10))
}

/// Frame used for modal dialogs.
pub fn modal_frame(t: &Tokens) -> Frame {
    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(t.radius_lg as u8))
        .inner_margin(Margin::same(18))
        .shadow(egui::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: t.shadow,
        })
}

/// Paint a hairline along the bottom edge of the last allocated rect.
pub fn bottom_hairline(ui: &Ui, rect: Rect, t: &Tokens) {
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, t.separator));
}

/// Paint a crisp vector sidebar icon showing split pane layout.
pub fn paint_sidebar_icon(
    painter: &egui::Painter,
    rect: Rect,
    stroke_color: Color32,
    fill_color: Color32,
    is_open: bool,
) {
    let box_rect = Rect::from_center_size(rect.center(), vec2(15.0, 13.0));
    let divider_x = box_rect.left() + 5.0;

    // Background fill for active sidebar pane
    if is_open {
        let left_pane = Rect::from_min_max(
            pos2(box_rect.left() + 1.0, box_rect.top() + 1.0),
            pos2(divider_x, box_rect.bottom() - 1.0),
        );
        painter.rect_filled(left_pane, CornerRadius::same(1), fill_color);
    }

    // Outer box stroke
    painter.rect_stroke(
        box_rect,
        CornerRadius::same(2),
        Stroke::new(1.2, stroke_color),
        egui::StrokeKind::Inside,
    );

    // Vertical dividing line
    painter.line_segment(
        [
            pos2(divider_x, box_rect.top() + 1.0),
            pos2(divider_x, box_rect.bottom() - 1.0),
        ],
        Stroke::new(1.2, stroke_color),
    );
}

/// Dedicated clickable vector sidebar toggle button.
pub fn icon_sidebar_button(ui: &mut Ui, t: &Tokens, is_open: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.is_pointer_button_down_on() {
            painter.rect_filled(rect, CornerRadius::same(t.radius_sm as u8), t.pressed);
        } else if resp.hovered() {
            painter.rect_filled(rect, CornerRadius::same(t.radius_sm as u8), t.hover);
        }

        let stroke_color = if resp.hovered() {
            t.text
        } else if is_open {
            t.accent_text
        } else {
            t.text_dim
        };

        let fill_color = if is_open {
            t.accent
        } else {
            Color32::TRANSPARENT
        };

        paint_sidebar_icon(painter, rect, stroke_color, fill_color, is_open);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Paint a clean vector search magnifying glass.
pub fn paint_search_icon(painter: &egui::Painter, center: egui::Pos2, size: f32, color: Color32) {
    let radius = size * 0.32;
    let circle_center = center - vec2(size * 0.12, size * 0.12);
    painter.circle_stroke(circle_center, radius, Stroke::new(1.3, color));

    let dir = vec2(0.7071, 0.7071);
    let start = circle_center + dir * radius;
    let end = circle_center + dir * (radius + size * 0.36);
    painter.line_segment([start, end], Stroke::new(1.5, color));
}

/// Vector search icon widget.
pub fn icon_search(ui: &mut Ui, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_search_icon(ui.painter(), rect.center(), size, color);
    }
    resp
}

/// Dedicated clickable vector close button ('×') that never produces tofu.
pub fn icon_close_button(ui: &mut Ui, t: &Tokens) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(rect, CornerRadius::same(t.radius_sm as u8), t.hover);
        }

        let color = if resp.hovered() { t.text } else { t.text_faint };
        let c = rect.center();
        let half = 3.8;
        painter.line_segment(
            [c - vec2(half, half), c + vec2(half, half)],
            Stroke::new(1.4, color),
        );
        painter.line_segment(
            [c + vec2(-half, half), c + vec2(half, -half)],
            Stroke::new(1.4, color),
        );
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Paint a clean vector location pin.
pub fn icon_pin(ui: &mut Ui, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center_x = rect.center().x;
        let head_y = rect.top() + size * 0.36;
        let radius = size * 0.30;
        let head_center = pos2(center_x, head_y);

        painter.circle_stroke(head_center, radius, Stroke::new(1.3, color));
        painter.circle_filled(head_center, size * 0.10, color);

        let bottom_tip = pos2(center_x, rect.bottom() - 0.5);
        let left_edge = head_center + vec2(-radius * 0.85, radius * 0.5);
        let right_edge = head_center + vec2(radius * 0.85, radius * 0.5);

        painter.line_segment([left_edge, bottom_tip], Stroke::new(1.2, color));
        painter.line_segment([right_edge, bottom_tip], Stroke::new(1.2, color));
    }
    resp
}

/// Paint a clean vector image/photo icon that never produces tofu.
pub fn icon_image(ui: &mut Ui, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let stroke = Stroke::new(1.2, color);
        let inner = rect.shrink(1.0);
        painter.rect_stroke(inner, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
        // Sun
        let sun_pos = pos2(inner.left() + inner.width() * 0.32, inner.top() + inner.height() * 0.35);
        painter.circle_filled(sun_pos, size * 0.12, color);
        // Mountains
        let p1 = pos2(inner.left() + 1.5, inner.bottom() - 1.5);
        let p2 = pos2(inner.left() + inner.width() * 0.45, inner.top() + inner.height() * 0.50);
        let p3 = pos2(inner.left() + inner.width() * 0.72, inner.bottom() - 1.5);
        painter.line_segment([p1, p2], stroke);
        painter.line_segment([p2, p3], stroke);
    }
    resp
}

/// Paint a crisp vector pencil icon for Notes mode.
pub fn paint_pencil_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let tip = pos2(center.x - 5.5, center.y + 5.5);
    let p_left = pos2(center.x - 2.5, center.y + 5.5);
    let p_right = pos2(center.x - 5.5, center.y + 2.5);
    let top_left = pos2(center.x + 3.0, center.y - 0.0);
    let top_right = pos2(center.x - 0.0, center.y - 3.0);
    let eraser_top_left = pos2(center.x + 5.0, center.y - 2.0);
    let eraser_top_right = pos2(center.x + 2.0, center.y - 5.0);

    let stroke = Stroke::new(1.3, color);
    // Shaft
    painter.line_segment([p_left, top_left], stroke);
    painter.line_segment([p_right, top_right], stroke);
    painter.line_segment([top_left, top_right], stroke);
    // Tip
    painter.line_segment([p_left, tip], stroke);
    painter.line_segment([p_right, tip], stroke);
    painter.circle_filled(tip + vec2(0.9, -0.9), 1.0, color);
    // Eraser
    painter.line_segment([top_left, eraser_top_left], stroke);
    painter.line_segment([top_right, eraser_top_right], stroke);
    painter.line_segment([eraser_top_left, eraser_top_right], stroke);
}

/// Paint a crisp vector calendar icon for Calendar mode.
pub fn paint_calendar_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let box_rect = Rect::from_center_size(center + vec2(0.0, 1.0), vec2(14.0, 12.0));
    painter.rect_stroke(box_rect, CornerRadius::same(2), Stroke::new(1.3, color), egui::StrokeKind::Inside);

    let bar_y = box_rect.top() + 3.5;
    painter.line_segment([pos2(box_rect.left(), bar_y), pos2(box_rect.right(), bar_y)], Stroke::new(1.1, color));

    // Top binder rings
    painter.line_segment(
        [pos2(box_rect.left() + 3.5, box_rect.top() - 2.0), pos2(box_rect.left() + 3.5, box_rect.top() + 1.5)],
        Stroke::new(1.4, color),
    );
    painter.line_segment(
        [pos2(box_rect.right() - 3.5, box_rect.top() - 2.0), pos2(box_rect.right() - 3.5, box_rect.top() + 1.5)],
        Stroke::new(1.4, color),
    );

    // Days grid dots
    painter.circle_filled(pos2(box_rect.left() + 3.5, bar_y + 3.0), 0.9, color);
    painter.circle_filled(pos2(box_rect.center().x, bar_y + 3.0), 0.9, color);
    painter.circle_filled(pos2(box_rect.right() - 3.5, bar_y + 3.0), 0.9, color);
    painter.circle_filled(pos2(box_rect.left() + 3.5, bar_y + 6.0), 0.9, color);
    painter.circle_filled(pos2(box_rect.center().x, bar_y + 6.0), 0.9, color);
    painter.circle_filled(pos2(box_rect.right() - 3.5, bar_y + 6.0), 0.9, color);
}

/// Paint a crisp vector graph constellation/network icon for Graph mode.
pub fn paint_graph_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let n_center = center;
    let n1 = pos2(center.x - 5.0, center.y - 4.5);
    let n2 = pos2(center.x + 5.0, center.y - 3.5);
    let n3 = pos2(center.x - 4.5, center.y + 4.5);
    let n4 = pos2(center.x + 4.5, center.y + 4.0);

    let edge_stroke = Stroke::new(1.1, color);
    let sub_stroke = Stroke::new(0.8, color.gamma_multiply(0.6));

    // Outer interconnects
    painter.line_segment([n1, n2], sub_stroke);
    painter.line_segment([n3, n4], sub_stroke);

    // Hub edges
    painter.line_segment([n_center, n1], edge_stroke);
    painter.line_segment([n_center, n2], edge_stroke);
    painter.line_segment([n_center, n3], edge_stroke);
    painter.line_segment([n_center, n4], edge_stroke);

    // Node circles
    painter.circle_filled(n_center, 2.2, color);
    painter.circle_filled(n1, 1.8, color);
    painter.circle_filled(n2, 1.8, color);
    painter.circle_filled(n3, 1.8, color);
    painter.circle_filled(n4, 1.8, color);
}

/// Paint a clean vector canvas icon.
#[allow(dead_code)]
pub fn paint_canvas_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let box_rect = Rect::from_center_size(center, vec2(14.0, 12.0));
    painter.rect_stroke(box_rect, CornerRadius::same(2), Stroke::new(1.2, color), egui::StrokeKind::Inside);
    let p_top = pos2(center.x, center.y - 3.5);
    let p_right = pos2(center.x + 3.5, center.y);
    let p_bottom = pos2(center.x, center.y + 3.5);
    let p_left = pos2(center.x - 3.5, center.y);
    let stroke = Stroke::new(1.0, color);
    painter.line_segment([p_top, p_right], stroke);
    painter.line_segment([p_right, p_bottom], stroke);
    painter.line_segment([p_bottom, p_left], stroke);
    painter.line_segment([p_left, p_top], stroke);
}

/// Paint a clean vector folder icon.
pub fn paint_folder_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    let min = rect.min;
    let size = rect.size();
    let body = Rect::from_min_max(
        pos2(min.x + size.x * 0.1, min.y + size.y * 0.32),
        pos2(min.x + size.x * 0.9, min.y + size.y * 0.82),
    );
    painter.rect_stroke(body, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
    // Tab on top
    painter.line_segment(
        [
            pos2(min.x + size.x * 0.1, min.y + size.y * 0.32),
            pos2(min.x + size.x * 0.1, min.y + size.y * 0.2),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(min.x + size.x * 0.1, min.y + size.y * 0.2),
            pos2(min.x + size.x * 0.45, min.y + size.y * 0.2),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(min.x + size.x * 0.45, min.y + size.y * 0.2),
            pos2(min.x + size.x * 0.55, min.y + size.y * 0.32),
        ],
        stroke,
    );
}

/// Icon-based segmented switcher for the main view modes (Notes pencil, Calendar, Graph).
pub fn top_mode_segmented(
    ui: &mut Ui,
    t: &Tokens,
    current: crate::state::TopBarMode,
) -> Option<crate::state::TopBarMode> {
    use crate::state::TopBarMode;
    let mut clicked = None;
    Frame::NONE
        .fill(t.field)
        .stroke(Stroke::new(1.0, t.separator))
        .corner_radius(CornerRadius::same(t.radius as u8))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                let modes = [
                    (TopBarMode::Notes, "Notes (Editor) — Ctrl+1", "notes"),
                    (TopBarMode::Calendar, "Calendar — Ctrl+2", "calendar"),
                    (TopBarMode::Graph, "Knowledge Graph — Ctrl+3", "graph"),
                ];

                for (mode, tooltip, icon_name) in modes {
                    let selected = mode == current;
                    let size = vec2(34.0, 26.0);
                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                    let radius = CornerRadius::same((t.radius as u8).saturating_sub(2));
                    let color = if selected {
                        ui.painter().rect_filled(rect, radius, t.card);
                        ui.painter().rect_stroke(
                            rect,
                            radius,
                            Stroke::new(1.0, t.card_border),
                            egui::StrokeKind::Inside,
                        );
                        t.accent_text
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, radius, t.hover);
                        t.text
                    } else {
                        t.text_dim
                    };

                    match icon_name {
                        "notes" => paint_pencil_icon(ui.painter(), rect, color),
                        "calendar" => paint_calendar_icon(ui.painter(), rect, color),
                        "graph" => paint_graph_icon(ui.painter(), rect, color),
                        _ => {}
                    }

                    if resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tooltip).clicked() {
                        clicked = Some(mode);
                    }
                }
            });
        });
    clicked
}

/// Actions triggered by the custom title bar window buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowControlAction {
    Minimize,
    MaximizeToggle,
    Close,
}

/// Paint a clean vector minimize icon (horizontal bar).
pub fn paint_window_minimize_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let y = center.y + 0.5;
    painter.line_segment(
        [pos2(center.x - 5.0, y), pos2(center.x + 5.0, y)],
        Stroke::new(1.3, color),
    );
}

/// Paint a clean vector maximize / restore icon.
pub fn paint_window_maximize_icon(
    painter: &egui::Painter,
    rect: Rect,
    color: Color32,
    bg_color: Color32,
    is_maximized: bool,
) {
    let center = rect.center();
    if is_maximized {
        // Restore icon: two overlapping squares
        let back_rect = Rect::from_center_size(center + vec2(1.5, -1.5), vec2(7.5, 7.5));
        painter.rect_stroke(back_rect, CornerRadius::ZERO, Stroke::new(1.1, color), egui::StrokeKind::Inside);
        let front_rect = Rect::from_center_size(center + vec2(-1.5, 1.5), vec2(7.5, 7.5));
        painter.rect_filled(front_rect, CornerRadius::ZERO, bg_color);
        painter.rect_stroke(front_rect, CornerRadius::ZERO, Stroke::new(1.1, color), egui::StrokeKind::Inside);
    } else {
        // Maximize icon: single square
        let box_rect = Rect::from_center_size(center, vec2(9.0, 9.0));
        painter.rect_stroke(box_rect, CornerRadius::ZERO, Stroke::new(1.2, color), egui::StrokeKind::Inside);
    }
}

/// Paint a clean vector close icon (✕).
pub fn paint_window_close_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let span = 4.2;
    let stroke = Stroke::new(1.25, color);
    painter.line_segment(
        [
            pos2(center.x - span, center.y - span),
            pos2(center.x + span, center.y + span),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(center.x - span, center.y + span),
            pos2(center.x + span, center.y - span),
        ],
        stroke,
    );
}

/// Render the trio of custom window control buttons (Close, Maximize/Restore, Minimize).
///
/// Designed to be called inside a `Layout::right_to_left` layout so Close is rightmost,
/// followed by Maximize, then Minimize.
pub fn window_control_buttons_rtl(
    ui: &mut Ui,
    t: &Tokens,
    is_maximized: bool,
) -> Option<WindowControlAction> {
    let mut action = None;
    let btn_size = vec2(32.0, 26.0);
    let radius = CornerRadius::same(t.radius_sm as u8);

    // 1. Close button (first in right_to_left -> rightmost position)
    let (close_rect, close_resp) = ui.allocate_exact_size(btn_size, Sense::click());
    if ui.is_rect_visible(close_rect) {
        let p = ui.painter();
        let close_danger = Color32::from_rgb(226, 45, 60);
        let close_pressed = Color32::from_rgb(186, 32, 45);
        let bg = if close_resp.is_pointer_button_down_on() {
            close_pressed
        } else if close_resp.hovered() {
            close_danger
        } else {
            Color32::TRANSPARENT
        };
        if bg != Color32::TRANSPARENT {
            p.rect_filled(close_rect, radius, bg);
        }
        let icon_color = if close_resp.hovered() {
            Color32::WHITE
        } else {
            t.text_dim
        };
        paint_window_close_icon(p, close_rect, icon_color);
    }
    if close_resp.on_hover_text("Close Window").clicked() {
        action = Some(WindowControlAction::Close);
    }

    ui.add_space(2.0);

    // 2. Maximize / Restore button (middle position)
    let (max_rect, max_resp) = ui.allocate_exact_size(btn_size, Sense::click());
    if ui.is_rect_visible(max_rect) {
        let p = ui.painter();
        let bg = if max_resp.is_pointer_button_down_on() {
            t.pressed
        } else if max_resp.hovered() {
            t.hover
        } else {
            Color32::TRANSPARENT
        };
        if bg != Color32::TRANSPARENT {
            p.rect_filled(max_rect, radius, bg);
        }
        let icon_color = if max_resp.hovered() { t.text } else { t.text_dim };
        let fill_bg = if bg != Color32::TRANSPARENT { bg } else { t.chrome };
        paint_window_maximize_icon(p, max_rect, icon_color, fill_bg, is_maximized);
    }
    let max_tip = if is_maximized { "Restore Window" } else { "Maximize Window" };
    if max_resp.on_hover_text(max_tip).clicked() {
        action = Some(WindowControlAction::MaximizeToggle);
    }

    ui.add_space(2.0);

    // 3. Minimize button (leftmost of the window control trio)
    let (min_rect, min_resp) = ui.allocate_exact_size(btn_size, Sense::click());
    if ui.is_rect_visible(min_rect) {
        let p = ui.painter();
        let bg = if min_resp.is_pointer_button_down_on() {
            t.pressed
        } else if min_resp.hovered() {
            t.hover
        } else {
            Color32::TRANSPARENT
        };
        if bg != Color32::TRANSPARENT {
            p.rect_filled(min_rect, radius, bg);
        }
        let icon_color = if min_resp.hovered() { t.text } else { t.text_dim };
        paint_window_minimize_icon(p, min_rect, icon_color);
    }
    if min_resp.on_hover_text("Minimize Window").clicked() {
        action = Some(WindowControlAction::Minimize);
    }

    action
}

/// Clickable pushpin button that never produces tofu.
pub fn icon_pin_button(ui: &mut Ui, t: &Tokens, is_pinned: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(rect, CornerRadius::same(t.radius_sm as u8), t.hover);
        }
        let color = if is_pinned {
            t.accent_text
        } else if resp.hovered() {
            t.text
        } else {
            t.text_faint
        };

        let cx = rect.center().x;
        let cy = rect.center().y;

        // Pushpin head
        let head_r = Rect::from_center_size(pos2(cx, cy - 3.5), vec2(7.0, 4.0));
        painter.rect(
            head_r,
            CornerRadius::same(1),
            if is_pinned { color } else { Color32::TRANSPARENT },
            Stroke::new(1.3, color),
            egui::StrokeKind::Inside,
        );
        // Base bar
        painter.line_segment([pos2(cx - 5.0, cy), pos2(cx + 5.0, cy)], Stroke::new(1.4, color));
        // Pin needle
        painter.line_segment([pos2(cx, cy), pos2(cx, cy + 5.5)], Stroke::new(1.4, color));
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Clickable palette icon (three color dots) that never produces tofu.
#[allow(dead_code)]
pub fn icon_palette_button(ui: &mut Ui, t: &Tokens) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(rect, CornerRadius::same(t.radius_sm as u8), t.hover);
        }
        let c = rect.center();
        painter.circle_filled(c + vec2(-4.0, -2.5), 2.5, Color32::from_rgb(235, 87, 87));
        painter.circle_filled(c + vec2(4.0, -2.5), 2.5, Color32::from_rgb(242, 201, 76));
        painter.circle_filled(c + vec2(0.0, 3.5), 2.5, Color32::from_rgb(45, 156, 219));
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Clickable vector trash can icon button.
pub fn icon_trash_button(ui: &mut Ui, t: &Tokens) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(rect, CornerRadius::same(t.radius_sm as u8), t.hover);
        }
        let color = if resp.hovered() { t.danger } else { t.text_faint };
        let cx = rect.center().x;
        let cy = rect.center().y;

        // Lid
        painter.line_segment([pos2(cx - 5.5, cy - 3.5), pos2(cx + 5.5, cy - 3.5)], Stroke::new(1.3, color));
        painter.line_segment([pos2(cx - 2.0, cy - 5.0), pos2(cx + 2.0, cy - 5.0)], Stroke::new(1.3, color));
        // Bin body
        let body = Rect::from_min_max(pos2(cx - 4.5, cy - 2.5), pos2(cx + 4.5, cy + 5.5));
        painter.rect_stroke(body, CornerRadius::same(1), Stroke::new(1.3, color), egui::StrokeKind::Inside);
        // Vertical lines inside bin
        painter.line_segment([pos2(cx - 1.5, cy - 0.5), pos2(cx - 1.5, cy + 3.5)], Stroke::new(1.0, color));
        painter.line_segment([pos2(cx + 1.5, cy - 0.5), pos2(cx + 1.5, cy + 3.5)], Stroke::new(1.0, color));
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Clickable vector checkbox that never produces tofu.
pub fn icon_checkbox(ui: &mut Ui, checked: bool, t: &Tokens) -> Response {
    let size = 15.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same(3);
        let border_color = if checked {
            t.accent
        } else if resp.hovered() {
            t.text
        } else {
            t.field_border
        };

        if checked {
            painter.rect_filled(rect, radius, t.accent);
            // Draw checkmark lines
            let p1 = rect.left_center() + vec2(3.0, 0.0);
            let p2 = rect.center() + vec2(-1.0, 3.5);
            let p3 = rect.right_top() + vec2(-3.5, 4.0);
            painter.line_segment([p1, p2], Stroke::new(1.8, Color32::WHITE));
            painter.line_segment([p2, p3], Stroke::new(1.8, Color32::WHITE));
        } else {
            painter.rect(
                rect,
                radius,
                if resp.hovered() { t.hover } else { t.field },
                Stroke::new(1.3, border_color),
                egui::StrokeKind::Inside,
            );
        }
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Clickable vector external/open icon button that never produces tofu.
pub fn icon_open_external_button(ui: &mut Ui, t: &Tokens) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(rect, CornerRadius::same(t.radius_sm as u8), t.hover);
        }
        let color = if resp.hovered() { t.text } else { t.text_dim };
        let cx = rect.center().x;
        let cy = rect.center().y;

        // Diagonal arrow pointing top-right
        let p_start = pos2(cx - 3.5, cy + 3.5);
        let p_end = pos2(cx + 3.5, cy - 3.5);
        painter.line_segment([p_start, p_end], Stroke::new(1.4, color));
        // Arrow head
        painter.line_segment([p_end, pos2(cx + 0.5, cy - 3.5)], Stroke::new(1.4, color));
        painter.line_segment([p_end, pos2(cx + 3.5, cy - 0.5)], Stroke::new(1.4, color));
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Install font fallback stack to prevent missing glyph tofu.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Check system fonts for extra fallback symbols/glyphs on NixOS and standard Linux
    let candidate_fonts = [
        "/run/current-system/sw/share/X11/fonts/DejaVuSans.ttf",
        "/run/current-system/sw/share/X11/fonts/unifont_upper.otf",
        "/run/current-system/sw/share/X11/fonts/NotoColorEmoji.ttf",
        "/run/current-system/sw/share/X11/fonts/NotoSansSymbols.ttf",
        "/run/current-system/sw/share/X11/fonts/NotoSansSymbols2-Regular.otf",
        "/run/current-system/sw/share/X11/fonts/unifont.otf",
        "/run/current-system/sw/share/X11/fonts/NotoSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    ];

    for (idx, path) in candidate_fonts.iter().enumerate() {
        if let Ok(bytes) = std::fs::read(path) {
            let name = format!("system_fallback_{idx}");
            fonts.font_data.insert(
                name.clone(),
                Arc::new(FontData::from_owned(bytes)),
            );
            if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
                prop.push(name.clone());
            }
            if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
                mono.push(name);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_leading_emoji() {
        assert_eq!(split_leading_emoji("📝 Note"), Some(("📝", "Note")));
        assert_eq!(split_leading_emoji("🚀 Launch"), Some(("🚀", "Launch")));
        assert_eq!(split_leading_emoji("🏷️ Tagged"), Some(("🏷️", "Tagged")));
        assert_eq!(split_leading_emoji("📅 2026-10-31"), Some(("📅", "2026-10-31")));
        assert_eq!(split_leading_emoji("🔔 Event"), Some(("🔔", "Event")));
        assert_eq!(split_leading_emoji("No emoji"), None);
        assert_eq!(split_leading_emoji(""), None);
    }
}
