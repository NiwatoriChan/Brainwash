//! Quick Notes view, masonry card board, and quick capture composer.

use egui::{
    vec2, Color32, CornerRadius, Frame, Id, Margin, RichText, ScrollArea, Sense,
    Stroke, Ui,
};
use mybrain_core::{Note, NoteId};

use crate::state::AppState;
use crate::theme::{self, Tokens};

pub struct QuickColorDef {
    pub name: &'static str,
    pub label: &'static str,
    pub dark_bg: Color32,
    pub light_bg: Color32,
    pub dot_color: Color32,
}

pub const QUICK_COLORS: &[QuickColorDef] = &[
    QuickColorDef {
        name: "default",
        label: "Default",
        dark_bg: Color32::from_rgb(28, 29, 34),
        light_bg: Color32::from_rgb(255, 255, 255),
        dot_color: Color32::from_rgb(140, 140, 145),
    },
    QuickColorDef {
        name: "coral",
        label: "Coral",
        dark_bg: Color32::from_rgb(95, 36, 28),
        light_bg: Color32::from_rgb(250, 222, 218),
        dot_color: Color32::from_rgb(235, 87, 87),
    },
    QuickColorDef {
        name: "peach",
        label: "Peach",
        dark_bg: Color32::from_rgb(90, 48, 22),
        light_bg: Color32::from_rgb(254, 239, 219),
        dot_color: Color32::from_rgb(242, 153, 74),
    },
    QuickColorDef {
        name: "sand",
        label: "Sand",
        dark_bg: Color32::from_rgb(85, 65, 20),
        light_bg: Color32::from_rgb(255, 248, 184),
        dot_color: Color32::from_rgb(242, 201, 76),
    },
    QuickColorDef {
        name: "mint",
        label: "Mint",
        dark_bg: Color32::from_rgb(28, 70, 44),
        light_bg: Color32::from_rgb(226, 246, 211),
        dot_color: Color32::from_rgb(39, 174, 96),
    },
    QuickColorDef {
        name: "teal",
        label: "Teal",
        dark_bg: Color32::from_rgb(22, 68, 72),
        light_bg: Color32::from_rgb(212, 244, 240),
        dot_color: Color32::from_rgb(45, 156, 219),
    },
    QuickColorDef {
        name: "sky",
        label: "Sky Blue",
        dark_bg: Color32::from_rgb(25, 55, 86),
        light_bg: Color32::from_rgb(218, 235, 252),
        dot_color: Color32::from_rgb(86, 204, 242),
    },
    QuickColorDef {
        name: "dusk",
        label: "Dusk",
        dark_bg: Color32::from_rgb(38, 44, 82),
        light_bg: Color32::from_rgb(222, 227, 251),
        dot_color: Color32::from_rgb(111, 82, 216),
    },
    QuickColorDef {
        name: "lavender",
        label: "Lavender",
        dark_bg: Color32::from_rgb(58, 36, 80),
        light_bg: Color32::from_rgb(241, 224, 251),
        dot_color: Color32::from_rgb(155, 81, 224),
    },
    QuickColorDef {
        name: "pink",
        label: "Blossom",
        dark_bg: Color32::from_rgb(76, 32, 58),
        light_bg: Color32::from_rgb(253, 222, 238),
        dot_color: Color32::from_rgb(235, 87, 137),
    },
    QuickColorDef {
        name: "clay",
        label: "Clay",
        dark_bg: Color32::from_rgb(54, 40, 32),
        light_bg: Color32::from_rgb(230, 201, 180),
        dot_color: Color32::from_rgb(180, 130, 100),
    },
];

pub fn card_bg_for_color(color_name: Option<&str>, is_dark: bool, default_bg: Color32) -> Color32 {
    let Some(name) = color_name else {
        return default_bg;
    };
    for def in QUICK_COLORS {
        if def.name.eq_ignore_ascii_case(name) {
            return if is_dark { def.dark_bg } else { def.light_bg };
        }
    }
    default_bg
}

enum QuickAction {
    TogglePin(NoteId),
    SetColor(NoteId, Option<String>),
    Delete(NoteId),
    OpenNote(NoteId),
    EditInPlace(NoteId),
    ToggleTask(NoteId, usize),
}

/// Render the Quick Notes interactive board in the CentralPanel.
pub fn show_keep_view(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());
    let mut actions: Vec<QuickAction> = Vec::new();

    ScrollArea::vertical()
        .id_salt("quick_notes_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(28, 20))
                .show(ui, |ui| {
                    let avail_w = ui.available_width();
                    let max_board_w = avail_w.clamp(240.0, 1200.0);
                    let side_pad = ((avail_w - max_board_w) * 0.5).max(0.0);

                    ui.horizontal(|ui| {
                        if side_pad > 0.0 {
                            ui.add_space(side_pad);
                        }
                        ui.vertical(|ui| {
                            ui.set_max_width(max_board_w);

                            // 1. "Take a note..." Creator Card at the top
                            ui.vertical_centered(|ui| {
                                render_composer_bar(ui, state, &t);
                            });

                            ui.add_space(24.0);

                            // Fetch and partition all quick notes
                            let all_notes = state.quick_config.all(&state.vault);

                            // Filter if search query is active
                            let filtered: Vec<&Note> = if !state.quick_search_query.trim().is_empty() {
                                let q = state.quick_search_query.trim().to_lowercase();
                                all_notes
                                    .into_iter()
                                    .filter(|n| {
                                        n.title().to_lowercase().contains(&q)
                                            || n.body().to_lowercase().contains(&q)
                                    })
                                    .collect()
                            } else {
                                all_notes
                            };

                            let mut pinned_notes: Vec<&Note> = Vec::new();
                            let mut other_notes: Vec<&Note> = Vec::new();

                            for note in filtered {
                                if note.frontmatter().get("pinned") == Some("true") {
                                    pinned_notes.push(note);
                                } else {
                                    other_notes.push(note);
                                }
                            }

                            if pinned_notes.is_empty() && other_notes.is_empty() {
                                render_empty_state(ui, &t);
                            } else {
                                // Pinned Notes Section
                                if !pinned_notes.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("PINNED")
                                                .size(11.0)
                                                .color(t.text_faint)
                                                .strong(),
                                        );
                                    });
                                    ui.add_space(6.0);
                                    render_masonry_grid(ui, &pinned_notes, &t, &mut actions);
                                    ui.add_space(24.0);
                                }

                                // Others Section
                                if !other_notes.is_empty() {
                                    if !pinned_notes.is_empty() {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new("OTHERS")
                                                    .size(11.0)
                                                    .color(t.text_faint)
                                                    .strong(),
                                            );
                                        });
                                        ui.add_space(6.0);
                                    }
                                    render_masonry_grid(ui, &other_notes, &t, &mut actions);
                                }
                            }
                        });
                    });
                    ui.add_space(20.0);
                });
        });

    // Process actions after UI traversal
    for action in actions {
        match action {
            QuickAction::TogglePin(id) => state.toggle_quick_pinned(&id),
            QuickAction::SetColor(id, col) => state.set_quick_color(&id, col.as_deref()),
            QuickAction::Delete(id) => state.delete_quick_note(&id),
            QuickAction::OpenNote(id) => state.open_note(id),
            QuickAction::EditInPlace(id) => state.open_quick_edit(id),
            QuickAction::ToggleTask(id, line_idx) => {
                toggle_task_line(state, &id, line_idx);
            }
        }
    }
}

/// Renders the "Take a note..." composer card at the top.
fn render_composer_bar(ui: &mut Ui, state: &mut AppState, t: &Tokens) {
    let composer_w = 580.0f32.min(ui.available_width() - 20.0);
    let is_dark = t.kind.is_dark();

    let bg = card_bg_for_color(state.quick_composer_color.as_deref(), is_dark, t.card);

    let card_frame = Frame::NONE
        .fill(bg)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 12));

    ui.allocate_ui_with_layout(
        vec2(composer_w, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            card_frame.show(ui, |ui| {
                ui.set_width(composer_w - 32.0);

                if !state.quick_composer_expanded {
                    // Collapsed state
                    ui.horizontal(|ui| {
                        let (rect, resp) = ui.allocate_exact_size(
                            vec2(ui.available_width() - 80.0, 24.0),
                            Sense::click(),
                        );
                        ui.painter().text(
                            rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            "Take a note…",
                            egui::FontId::proportional(14.0),
                            t.text_faint,
                        );

                        if resp.clicked() {
                            state.quick_composer_expanded = true;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if theme::ghost_button(
                                ui,
                                t,
                                RichText::new("+ Checklist").size(12.0).color(t.text_dim),
                            )
                            .on_hover_text("New checklist note")
                            .clicked()
                            {
                                state.quick_composer_expanded = true;
                                if state.quick_capture_buffer.is_empty() {
                                    state.quick_capture_buffer = "- [ ] ".to_string();
                                }
                            }

                            if theme::icon_pin_button(ui, t, state.quick_composer_pinned)
                                .on_hover_text(if state.quick_composer_pinned { "Unpin note" } else { "Pin note" })
                                .clicked()
                            {
                                state.quick_composer_pinned = !state.quick_composer_pinned;
                                state.quick_composer_expanded = true;
                            }
                        });
                    });
                } else {
                    // Expanded composer
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.quick_composer_title)
                                .hint_text("Title")
                                .font(egui::FontId::proportional(15.0))
                                .frame(Frame::NONE)
                                .desired_width(ui.available_width() - 36.0),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if theme::icon_pin_button(ui, t, state.quick_composer_pinned)
                                .on_hover_text(if state.quick_composer_pinned { "Unpin note" } else { "Pin note" })
                                .clicked()
                            {
                                state.quick_composer_pinned = !state.quick_composer_pinned;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Body multiline text edit
                    let edit = ui.add(
                        egui::TextEdit::multiline(&mut state.quick_capture_buffer)
                            .hint_text("Take a note… (supports Markdown and [[links]])")
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .desired_rows(4),
                    );
                    edit.request_focus();

                    ui.add_space(10.0);

                    // Action row
                    ui.horizontal(|ui| {
                        // Color picker dots
                        if let Some(action) = theme::color_picker_dots(ui, t, state.settings.effective_color_palette(), state.quick_composer_color.as_deref()) {
                            match action {
                                theme::ColorPickerAction::SetColor(c) => {
                                    state.quick_composer_color = c;
                                }
                                theme::ColorPickerAction::OpenCustomColor => {
                                    let curr = state.quick_composer_color.clone();
                                    state.open_custom_color_picker(crate::state::CustomColorTarget::QuickComposer, curr.as_deref());
                                }
                            }
                        }

                        ui.add_space(6.0);

                        // Add checklist item button
                        if theme::ghost_button(
                            ui,
                            t,
                            RichText::new("+ Checkbox").size(11.5).color(t.text_dim),
                        )
                        .clicked()
                        {
                            if !state.quick_capture_buffer.is_empty()
                                && !state.quick_capture_buffer.ends_with('\n')
                            {
                                state.quick_capture_buffer.push('\n');
                            }
                            state.quick_capture_buffer.push_str("- [ ] ");
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Close")
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(t.accent)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                                )
                                .clicked()
                            {
                                state.capture_quick_keep();
                            }

                            if theme::ghost_button(
                                ui,
                                t,
                                RichText::new("Cancel").color(t.text_faint),
                            )
                            .clicked()
                            {
                                state.quick_composer_expanded = false;
                                state.quick_capture_buffer.clear();
                                state.quick_composer_title.clear();
                            }
                        });
                    });
                }
            });
        },
    );
}

/// Renders a multi-column responsive grid (Google Keep masonry look).
fn render_masonry_grid(
    ui: &mut Ui,
    notes: &[&Note],
    t: &Tokens,
    actions: &mut Vec<QuickAction>,
) {
    let avail_w = ui.available_width().max(300.0);
    let col_w = 260.0f32;
    let col_gap = 14.0f32;

    let col_count = (((avail_w + col_gap) / (col_w + col_gap)).floor() as usize).clamp(1, 4);

    let mut columns: Vec<Vec<&Note>> = vec![Vec::new(); col_count];
    for (idx, note) in notes.iter().enumerate() {
        columns[idx % col_count].push(*note);
    }

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = col_gap;

        for col_notes in columns {
            ui.vertical(|ui| {
                ui.set_width(col_w);
                ui.spacing_mut().item_spacing.y = col_gap;

                for note in col_notes {
                    render_keep_card(ui, note, col_w, t, actions);
                }
            });
        }
    });
}

/// Renders an individual Quick Note card with pure vector buttons.
fn render_keep_card(
    ui: &mut Ui,
    note: &Note,
    width: f32,
    t: &Tokens,
    actions: &mut Vec<QuickAction>,
) {
    let fm = note.frontmatter();
    let is_pinned = fm.get("pinned") == Some("true");
    let color_name = fm.get("color");
    let bg = card_bg_for_color(color_name, t.kind.is_dark(), t.card);

    let note_id = note.id.clone();
    let title = fm
        .get("title")
        .map(|s| s.to_string())
        .unwrap_or_else(|| note.title().to_string());

    let raw_body = note.body();
    // Clean out redundant "# Title" if present in body
    let body_text = raw_body
        .trim_start_matches(|c: char| c == '#' || c.is_whitespace() || c == '\n')
        .trim();

    let frame = Frame::NONE
        .fill(bg)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14));

    let resp = frame.show(ui, |ui| {
        ui.set_width(width - 28.0);

        // Header: Title and Pin button
        ui.horizontal(|ui| {
            if !title.is_empty() {
                ui.label(RichText::new(&title).strong().size(14.0).color(t.text));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_pin_button(ui, t, is_pinned)
                    .on_hover_text(if is_pinned { "Unpin note" } else { "Pin note" })
                    .clicked()
                {
                    actions.push(QuickAction::TogglePin(note_id.clone()));
                }
            });
        });

        ui.add_space(4.0);

        // Body: Checkboxes or readable snippet
        let lines: Vec<&str> = body_text.lines().collect();
        let has_tasks = lines.iter().any(|l| l.starts_with("- [ ] ") || l.starts_with("- [x] "));

        if has_tasks {
            for (idx, line) in lines.iter().take(6).enumerate() {
                if line.starts_with("- [ ] ") || line.starts_with("- [x] ") {
                    let done = line.starts_with("- [x] ");
                    let item_text = &line[6..];

                    ui.horizontal(|ui| {
                        if theme::icon_checkbox(ui, done, t).clicked() {
                            actions.push(QuickAction::ToggleTask(note_id.clone(), idx));
                        }

                        let text_color = if done { t.text_faint } else { t.text };
                        let mut rt = RichText::new(item_text).size(12.0).color(text_color);
                        if done {
                            rt = rt.strikethrough();
                        }
                        ui.label(rt);
                    });
                } else if !line.trim().is_empty() {
                    ui.label(RichText::new(*line).size(12.0).color(t.text_dim));
                }
            }
        } else if !body_text.is_empty() {
            let max_body_lines = 8;
            let display_text = lines
                .iter()
                .take(max_body_lines)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");

            ui.label(RichText::new(display_text).size(12.5).color(t.text_dim));

            if lines.len() > max_body_lines {
                ui.label(RichText::new("…").size(11.0).color(t.text_faint));
            }
        }

        ui.add_space(8.0);

        // Bottom Card Action Toolbar with pure vector icons
        ui.horizontal(|ui| {
            // Vector color palette menu button
            let menu_resp = ui.menu_button("   ", |ui| {
                ui.set_max_width(180.0);
                ui.horizontal_wrapped(|ui| {
                    for color_def in QUICK_COLORS {
                        let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                        ui.painter().circle_filled(rect.center(), 7.0, color_def.dot_color);
                        if resp.on_hover_text(color_def.label).clicked() {
                            let col = if color_def.name == "default" {
                                None
                            } else {
                                Some(color_def.name.to_string())
                            };
                            actions.push(QuickAction::SetColor(note_id.clone(), col));
                            ui.close();
                        }
                    }
                });
            });
            let c = menu_resp.response.rect.center();
            ui.painter().circle_filled(c + vec2(-4.0, -2.5), 2.5, Color32::from_rgb(235, 87, 87));
            ui.painter().circle_filled(c + vec2(4.0, -2.5), 2.5, Color32::from_rgb(242, 201, 76));
            ui.painter().circle_filled(c + vec2(0.0, 3.5), 2.5, Color32::from_rgb(45, 156, 219));
            menu_resp.response.on_hover_text("Change note color");

            // Vector external/open in editor button
            if theme::icon_open_external_button(ui, t)
                .on_hover_text("Open in editor")
                .clicked()
            {
                actions.push(QuickAction::OpenNote(note_id.clone()));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Vector trash delete button
                if theme::icon_trash_button(ui, t)
                    .on_hover_text("Delete quick note")
                    .clicked()
                {
                    actions.push(QuickAction::Delete(note_id.clone()));
                }
            });
        });
    });

    if resp.response.clicked() {
        actions.push(QuickAction::EditInPlace(note.id.clone()));
    }
}

/// Toggles task checkbox on a specific line of a note.
fn toggle_task_line(state: &mut AppState, id: &NoteId, line_idx: usize) {
    if let Some(note) = state.vault.get(id) {
        let mut lines: Vec<String> = note.content.lines().map(|s| s.to_string()).collect();
        // Skip frontmatter lines
        let mut body_line_count = 0;
        let mut in_fm = false;
        let mut target_idx = None;

        for (i, line) in lines.iter().enumerate() {
            if line.trim() == "---" {
                in_fm = !in_fm;
                continue;
            }
            if !in_fm {
                if body_line_count == line_idx {
                    target_idx = Some(i);
                    break;
                }
                body_line_count += 1;
            }
        }

        if let Some(idx) = target_idx {
            if lines[idx].starts_with("- [ ] ") {
                lines[idx] = lines[idx].replacen("- [ ] ", "- [x] ", 1);
            } else if lines[idx].starts_with("- [x] ") {
                lines[idx] = lines[idx].replacen("- [x] ", "- [ ] ", 1);
            }
            let updated = lines.join("\n") + "\n";
            let _ = state.vault.save(id, &updated);
        }
    }
}

/// In-place Quick note editing modal dialog.
pub fn show_keep_edit_dialog(ctx: &egui::Context, state: &mut AppState) {
    if state.quick_edit_note.is_none() {
        return;
    }

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut should_save = false;
    let mut should_open_editor = false;

    let is_dark = t.kind.is_dark();
    let bg = card_bg_for_color(state.quick_edit_color.as_deref(), is_dark, t.card);

    let modal = egui::Modal::new(Id::unique("quick_edit_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(
            Frame::NONE
                .fill(bg)
                .stroke(Stroke::new(1.0, t.card_border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::same(20)),
        );

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(520.0);

        // Header
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.quick_edit_title)
                    .hint_text("Title")
                    .font(egui::FontId::proportional(16.0))
                    .frame(Frame::NONE)
                    .desired_width(ui.available_width() - 36.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_pin_button(ui, &t, state.quick_edit_pinned)
                    .on_hover_text(if state.quick_edit_pinned { "Unpin note" } else { "Pin note" })
                    .clicked()
                {
                    state.quick_edit_pinned = !state.quick_edit_pinned;
                }
            });
        });

        ui.add_space(10.0);

        // Multiline body
        ui.add(
            egui::TextEdit::multiline(&mut state.quick_edit_body)
                .hint_text("Note content…")
                .frame(Frame::NONE)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );

        ui.add_space(14.0);

        // Footer action bar
        ui.horizontal(|ui| {
            // Color picker dots
            for color_def in QUICK_COLORS {
                let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                let is_curr = state.quick_edit_color.as_deref() == Some(color_def.name)
                    || (state.quick_edit_color.is_none() && color_def.name == "default");

                ui.painter().circle_filled(rect.center(), 7.0, color_def.dot_color);
                if is_curr {
                    ui.painter().circle_stroke(rect.center(), 8.5, Stroke::new(1.5, t.text));
                }
                if resp.on_hover_text(color_def.label).clicked() {
                    state.quick_edit_color = if color_def.name == "default" {
                        None
                    } else {
                        Some(color_def.name.to_string())
                    };
                }
            }

            ui.add_space(8.0);

            if theme::ghost_button(ui, &t, RichText::new("Open in Editor").size(12.0).color(t.text_dim)).clicked() {
                should_save = true;
                should_open_editor = true;
                should_close = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Close").strong().color(Color32::WHITE))
                            .fill(t.accent)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    )
                    .clicked()
                {
                    should_save = true;
                    should_close = true;
                }
            });
        });
    });

    if should_save {
        let note_id = state.quick_edit_note.clone();
        state.save_quick_edit();
        if should_open_editor {
            if let Some(id) = note_id {
                state.open_note(id);
                state.top_mode = crate::state::TopBarMode::Notes;
                state.sidebar_section = crate::state::SidebarSection::Notes;
            }
        }
    }
    if modal_resp.should_close() || should_close {
        state.quick_edit_note = None;
    }
}

/// Fallback quick capture modal launcher.
pub fn show(_ctx: &egui::Context, state: &mut AppState) {
    if state.show_quick_capture {
        state.sidebar_section = crate::state::SidebarSection::Quick;
        state.top_mode = crate::state::TopBarMode::Notes;
        state.quick_composer_expanded = true;
        state.show_quick_capture = false;
    }
}

fn render_empty_state(ui: &mut Ui, t: &Tokens) {
    ui.add_space(40.0);
    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(t.radius as u8))
        .inner_margin(Margin::same(28))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("No quick notes here yet")
                        .strong()
                        .size(15.0)
                        .color(t.text),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Notes you add in the box above will appear here.")
                        .color(t.text_dim)
                        .size(12.5),
                );
            });
        });
}
