use egui::{
    text::LayoutJob, CornerRadius, FontId, Frame, Margin, RichText, ScrollArea, Stroke,
    TextBuffer, TextFormat, Ui,
};
use mybrain_core::markdown::{
    parse_blocks, parse_inline, toggle_task, Block, Inline, ListMarker,
};

use crate::state::{AppState, ViewMode};
use crate::theme::{
    ACCENT_CYAN, ACCENT_PURPLE, ACCENT_PURPLE_LIGHT, BG_CANVAS, BG_CARD, BG_HOVER, BORDER_COLOR,
    TEXT_FAINT, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let Some(active_id) = state.active_note.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("No note selected.\nCreate or choose a note from the sidebar.").color(TEXT_MUTED).size(16.0));
        });
        return;
    };

    // Note header / Toolbar
    ui.horizontal(|ui| {
        // Back / Forward buttons
        if ui.add_enabled(state.can_go_back(), egui::Button::new("◀").frame(false)).clicked() {
            state.go_back();
        }
        if ui.add_enabled(state.can_go_forward(), egui::Button::new("▶").frame(false)).clicked() {
            state.go_forward();
        }

        ui.separator();

        // Note title & folder
        ui.vertical(|ui| {
            if let Some(folder) = active_id.folder() {
                ui.label(RichText::new(folder).color(TEXT_FAINT).size(11.0));
            }
            ui.heading(RichText::new(active_id.name()).color(TEXT_PRIMARY).strong());
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Delete button
            if ui.button(RichText::new("🗑").color(TEXT_MUTED)).on_hover_text("Delete note").clicked() {
                state.delete_current_note();
                return;
            }

            ui.separator();

            // View mode switchers
            let mode = state.view_mode;
            if ui.selectable_label(mode == ViewMode::PreviewOnly, "👁 Preview").clicked() {
                state.view_mode = ViewMode::PreviewOnly;
            }
            if ui.selectable_label(mode == ViewMode::Split, "🌓 Split").clicked() {
                state.view_mode = ViewMode::Split;
            }
            if ui.selectable_label(mode == ViewMode::EditOnly, "✏ Edit").clicked() {
                state.view_mode = ViewMode::EditOnly;
            }

            // Save status indicator
            if state.editor_dirty {
                ui.label(RichText::new("● Unsaved").color(ACCENT_PURPLE_LIGHT).size(11.0));
            } else {
                ui.label(RichText::new("✓ Saved").color(TEXT_FAINT).size(11.0));
            }
        });
    });

    ui.separator();

    // Main workspace area
    let available_width = ui.available_width();
    match state.view_mode {
        ViewMode::EditOnly => {
            render_editor(ui, state, available_width);
        }
        ViewMode::PreviewOnly => {
            render_preview(ui, state, available_width);
        }
        ViewMode::Split => {
            let half = (available_width - 16.0) / 2.0;
            ui.columns(2, |cols| {
                render_editor(&mut cols[0], state, half);
                render_preview(&mut cols[1], state, half);
            });
        }
    }
}

fn render_editor(ui: &mut Ui, state: &mut AppState, width: f32) {
    Frame::NONE
        .fill(BG_CANVAS)
        .inner_margin(Margin::same(8))
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .show(ui, |ui| {
            ui.set_width(width);
            ScrollArea::vertical().id_salt("editor_scroll").show(ui, |ui| {
                let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
                    let mut layout_job = highlight_markdown(buf.as_str());
                    layout_job.wrap.max_width = wrap_width;
                    ui.ctx().fonts_mut(|f| f.layout_job(layout_job))
                };

                let output = egui::TextEdit::multiline(&mut state.editor_buffer)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(35)
                    .layouter(&mut layouter)
                    .frame(Frame::NONE)
                    .show(ui);

                if output.response.changed() {
                    state.editor_dirty = true;
                }

                // Wiki-link autocomplete popup when typing `[[`
                render_link_autocomplete(ui, state, &output);
            });
        });
}

fn render_link_autocomplete(
    ui: &mut Ui,
    state: &mut AppState,
    output: &egui::text_edit::TextEditOutput,
) {
    if let Some(cursor_range) = output.cursor_range {
        let cursor_char_idx = cursor_range.primary.index.0;
        let text = state.editor_buffer.clone();

        // Convert char offset to byte offset
        let byte_offset = text
            .char_indices()
            .nth(cursor_char_idx)
            .map(|(b, _)| b)
            .unwrap_or(text.len());

        if let Some((start_pos, query)) = mybrain_core::links::open_link_at(&text, byte_offset) {
            let query_lower = query.to_lowercase();
            let matches: Vec<String> = state
                .vault
                .notes()
                .filter(|n| n.id.as_str().to_lowercase().contains(&query_lower))
                .take(6)
                .map(|n| n.id.as_str().to_string())
                .collect();

            if !matches.is_empty() {
                let mut chosen_note = None;
                Frame::NONE
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.5, ACCENT_PURPLE))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(6))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Link to note:").color(TEXT_MUTED).size(11.0));
                        for note_name in matches {
                            if ui.button(RichText::new(format!("📄 [[{}]]", note_name)).color(ACCENT_PURPLE_LIGHT)).clicked() {
                                chosen_note = Some(note_name);
                            }
                        }
                    });

                if let Some(note_name) = chosen_note {
                    let mut new_text = text[..start_pos].to_string();
                    new_text.push_str(&note_name);
                    new_text.push_str("]]");
                    new_text.push_str(&text[byte_offset..]);
                    state.editor_buffer = new_text;
                    state.editor_dirty = true;
                }
            }
        }
    }
}

fn render_preview(ui: &mut Ui, state: &mut AppState, width: f32) {
    Frame::NONE
        .fill(BG_CANVAS)
        .inner_margin(Margin::same(12))
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .show(ui, |ui| {
            ui.set_width(width);
            ScrollArea::vertical().id_salt("preview_scroll").show(ui, |ui| {
                let blocks = parse_blocks(&state.editor_buffer);
                let mut task_to_toggle: Option<usize> = None;
                let mut note_to_open: Option<String> = None;

                for item in blocks {
                    let line_no = item.line;
                    match item.block {
                        Block::Heading { level, text } => {
                            let size = match level {
                                1 => 24.0,
                                2 => 20.0,
                                3 => 17.0,
                                _ => 14.0,
                            };
                            ui.add_space(8.0);
                            ui.label(RichText::new(text).size(size).color(TEXT_PRIMARY).strong());
                            if level <= 2 {
                                ui.separator();
                            }
                        }
                        Block::Paragraph(text) => {
                            render_inline_spans(ui, text, &mut note_to_open);
                        }
                        Block::ListItem { indent, marker, text } => {
                            ui.horizontal(|ui| {
                                if indent > 0 {
                                    ui.add_space(indent as f32 * 16.0);
                                }
                                match marker {
                                    ListMarker::Bullet => {
                                        ui.label(RichText::new("•").color(ACCENT_PURPLE_LIGHT));
                                    }
                                    ListMarker::Ordered(n) => {
                                        ui.label(RichText::new(format!("{n}.")).color(TEXT_MUTED));
                                    }
                                    ListMarker::Task { checked } => {
                                        let mut is_checked = checked;
                                        if ui.checkbox(&mut is_checked, "").changed() {
                                            task_to_toggle = Some(line_no);
                                        }
                                    }
                                }
                                render_inline_spans(ui, text, &mut note_to_open);
                            });
                        }
                        Block::Quote(text) => {
                            Frame::NONE
                                .fill(BG_CARD)
                                .stroke(Stroke::new(1.0, BORDER_COLOR))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("▎").color(ACCENT_PURPLE));
                                        render_inline_spans(ui, text, &mut note_to_open);
                                    });
                                });
                        }
                        Block::Code { lang, code } => {
                            Frame::NONE
                                .fill(BG_CARD)
                                .stroke(Stroke::new(1.0, BORDER_COLOR))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(8))
                                .show(ui, |ui| {
                                    if !lang.is_empty() {
                                        ui.label(RichText::new(lang).color(TEXT_FAINT).size(10.0));
                                    }
                                    ui.label(RichText::new(code.trim_end()).monospace().color(TEXT_PRIMARY));
                                });
                        }
                        Block::Rule => {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);
                        }
                        Block::Blank => {
                            ui.add_space(4.0);
                        }
                    }
                }

                // If user toggled a task in preview mode
                if let Some(line) = task_to_toggle {
                    if let Some(new_content) = toggle_task(&state.editor_buffer, line) {
                        state.editor_buffer = new_content;
                        state.editor_dirty = true;
                    }
                }

                // If user clicked a [[wiki link]] in preview mode
                if let Some(target) = note_to_open {
                    state.open_or_create(&target);
                }
            });
        });
}

fn render_inline_spans(ui: &mut Ui, text: &str, note_to_open: &mut Option<String>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let spans = parse_inline(text);
        for span in spans {
            match span {
                Inline::Text(t) => {
                    ui.label(RichText::new(t).color(TEXT_PRIMARY));
                }
                Inline::Bold(t) => {
                    ui.label(RichText::new(t).strong().color(TEXT_PRIMARY));
                }
                Inline::Italic(t) => {
                    ui.label(RichText::new(t).italics().color(TEXT_PRIMARY));
                }
                Inline::Code(t) => {
                    Frame::NONE
                        .fill(BG_CARD)
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(Margin::symmetric(4, 1))
                        .show(ui, |ui| {
                            ui.label(RichText::new(t).monospace().color(ACCENT_CYAN).size(12.0));
                        });
                }
                Inline::Url(u) => {
                    ui.hyperlink_to(RichText::new(u).color(ACCENT_CYAN).underline(), u);
                }
                Inline::Tag(tag) => {
                    Frame::NONE
                        .fill(BG_HOVER)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(RichText::new(format!("#{tag}")).color(ACCENT_PURPLE_LIGHT).size(11.0));
                        });
                }
                Inline::Link(link) => {
                    let label = link.display_text();
                    let btn = ui.button(
                        RichText::new(format!("[[{label}]]"))
                            .color(ACCENT_PURPLE_LIGHT)
                            .underline(),
                    );
                    if btn.on_hover_text(format!("Jump to note: {}", link.target)).clicked() {
                        *note_to_open = Some(link.target.clone());
                    }
                }
            }
        }
    });
}

fn highlight_markdown(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let default_font = FontId::monospace(13.5);
    let heading_font = FontId::monospace(15.0);

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            job.append(
                line,
                0.0,
                TextFormat {
                    font_id: heading_font.clone(),
                    color: ACCENT_PURPLE_LIGHT,
                    ..Default::default()
                },
            );
        } else if trimmed.starts_with("```") {
            job.append(
                line,
                0.0,
                TextFormat {
                    font_id: default_font.clone(),
                    color: ACCENT_CYAN,
                    ..Default::default()
                },
            );
        } else {
            // Highlight [[wiki links]] within the line
            let mut last = 0;
            for link in mybrain_core::links::parse_links(line) {
                if link.range.start > last {
                    job.append(
                        &line[last..link.range.start],
                        0.0,
                        TextFormat {
                            font_id: default_font.clone(),
                            color: TEXT_PRIMARY,
                            ..Default::default()
                        },
                    );
                }
                job.append(
                    &line[link.range.clone()],
                    0.0,
                    TextFormat {
                        font_id: default_font.clone(),
                        color: ACCENT_PURPLE,
                        ..Default::default()
                    },
                );
                last = link.range.end;
            }
            if last < line.len() {
                job.append(
                    &line[last..],
                    0.0,
                    TextFormat {
                        font_id: default_font.clone(),
                        color: TEXT_PRIMARY,
                        ..Default::default()
                    },
                );
            }
        }
    }
    job
}
