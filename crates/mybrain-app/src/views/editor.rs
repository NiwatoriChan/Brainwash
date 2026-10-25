//! Markdown editor and Obsidian-style live rendering view.

use egui::{
    text::LayoutJob, Color32, CornerRadius, FontId, Frame, Margin, RichText, ScrollArea, Stroke,
    TextBuffer, TextFormat, Ui,
};
use mybrain_core::frontmatter::Frontmatter;
use mybrain_core::markdown::{
    parse_blocks, parse_inline, toggle_task, Block, Inline, ListMarker,
};
use mybrain_core::vault::Vault;
use mybrain_core::NoteId;

use crate::state::{AppState, ViewMode};
use crate::theme::{self, Tokens};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());

    let Some(active_id) = state.active_note.clone() else {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("No note selected")
                        .color(t.text_dim)
                        .size(17.0)
                        .strong(),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Choose a note from the sidebar or press Ctrl+N to create one.")
                        .color(t.text_faint)
                        .size(13.0),
                );
            });
        });
        return;
    };

    // Note header / Toolbar
    Frame::NONE
        .fill(t.canvas)
        .inner_margin(Margin::symmetric(24, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Navigation history
                let can_back = state.can_go_back();
                let can_forward = state.can_go_forward();
                if ui.add_enabled(can_back, egui::Button::new("←").frame(false)).clicked() {
                    state.go_back();
                }
                if ui.add_enabled(can_forward, egui::Button::new("→").frame(false)).clicked() {
                    state.go_forward();
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // Note breadcrumb & title
                ui.horizontal(|ui| {
                    if let Some(folder) = active_id.folder() {
                        theme::pill(ui, folder, t.text_faint, t.field);
                        ui.label(RichText::new("/").color(t.text_faint).size(12.0));
                    }
                    ui.label(RichText::new(active_id.name()).color(t.text).strong().size(15.0));

                    // Daily Note badge & jump to Calendar
                    if let Some(daily_date) = state.daily_config.date_of(&active_id) {
                        ui.add_space(4.0);
                        theme::pill(ui, "Daily Note", t.accent_text, t.accent_soft);

                        let events_count = state.event_store.on(&state.vault, daily_date).len();
                        if events_count > 0 {
                            theme::pill(ui, &format!("{events_count} event{}", if events_count == 1 { "" } else { "s" }), t.warning, t.accent_soft);
                        }

                        if theme::ghost_button(ui, &t, RichText::new("+ Event").size(11.5).color(t.success))
                            .on_hover_text("Schedule an event for this date")
                            .clicked()
                        {
                            state.new_event_draft = mybrain_core::EventDraft::new(daily_date);
                            state.show_new_event_dialog = true;
                        }

                        if events_count > 0 {
                            if theme::ghost_button(ui, &t, RichText::new("Sync Events").size(11.5).color(t.accent_text))
                                .on_hover_text("Synchronize scheduled events into note body")
                                .clicked()
                            {
                                state.sync_events_to_daily_note(daily_date);
                            }
                        }

                        if theme::ghost_button(ui, &t, RichText::new("Calendar ↗").size(11.5).color(t.accent_text))
                            .on_hover_text("View in Daily Calendar")
                            .clicked()
                        {
                            state.selected_daily_date = daily_date;
                            state.sidebar_section = crate::state::SidebarSection::Daily;
                        }
                    }

                    // Event badge & jump to Agenda
                    let event_info = state.vault.get(&active_id).and_then(|n| mybrain_core::Event::from_note(n));
                    if let Some(ev) = event_info {
                        ui.add_space(4.0);
                        theme::pill(ui, &format!("Event: {}", ev.time_label()), t.warning, t.accent_soft);
                        if theme::ghost_button(ui, &t, RichText::new("Agenda ↗").size(11.5).color(t.warning))
                            .on_hover_text("View in Events Agenda")
                            .clicked()
                        {
                            state.sidebar_section = crate::state::SidebarSection::Events;
                        }
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Delete button
                    if theme::ghost_button(ui, &t, RichText::new("Delete").color(t.text_faint).size(12.0))
                        .on_hover_text("Delete this note")
                        .clicked()
                    {
                        state.delete_current_note();
                        return;
                    }

                    // + Insert Object dropdown menu
                    ui.menu_button(
                        RichText::new("+ Insert ▾").color(t.accent_text).size(12.0).strong(),
                        |ui| {
                            ui.set_min_width(175.0);
                            if ui.button("📅 Event Object").clicked() {
                                let today = chrono::Local::now().date_naive();
                                let snippet = format!(
                                    "```event\ntitle: New Event\ndate: {}\ntime: 10:00 - 11:00\nlocation: Room A\ndescription: Agenda details\n```\n",
                                    today.format("%Y-%m-%d")
                                );
                                state.insert_snippet_into_active_note(&snippet);
                                ui.close();
                            }
                            if ui.button("🔍 Query Object").clicked() {
                                let snippet = "```query\nevents\n```\n";
                                state.insert_snippet_into_active_note(snippet);
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("☑ Task Item").clicked() {
                                state.insert_snippet_into_active_note("- [ ] ");
                                ui.close();
                            }
                            if ui.button("🔗 Note Link").clicked() {
                                state.insert_snippet_into_active_note("[[]]");
                                ui.close();
                            }
                            if ui.button("🖼 Image").clicked() {
                                state.insert_snippet_into_active_note("![Alt text](image.png)");
                                ui.close();
                            }
                            if ui.button("💡 Note Callout").clicked() {
                                state.insert_snippet_into_active_note("> [!NOTE]\n> Key takeaway or important reminder.\n");
                                ui.close();
                            }
                            if ui.button("📊 Table").clicked() {
                                state.insert_snippet_into_active_note("| Column 1 | Column 2 |\n| --- | --- |\n| Value 1 | Value 2 |\n");
                                ui.close();
                            }
                        },
                    );

                    ui.separator();

                    // View mode segmented toggle
                    let mode = state.view_mode;
                    if let Some(new_mode) = theme::segmented(
                        ui,
                        &t,
                        mode,
                        &[
                            (ViewMode::EditOnly, "Edit"),
                            (ViewMode::Split, "Split"),
                            (ViewMode::PreviewOnly, "Preview"),
                        ],
                    ) {
                        state.view_mode = new_mode;
                    }

                    ui.separator();

                    // Save button / status indicator
                    if state.editor_dirty {
                        if ui.add(
                            egui::Button::new(RichText::new("Save (Ctrl+S)").color(Color32::WHITE).size(11.5).strong())
                                .fill(t.accent)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                        ).clicked() {
                            state.save_current();
                        }
                    }
                });
            });
        });

    theme::bottom_hairline(ui, ui.max_rect(), &t);

    // Main workspace area
    let available_size = ui.available_size();
    match state.view_mode {
        ViewMode::EditOnly => {
            ui.allocate_ui_with_layout(
                available_size,
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    render_editor_pane(ui, state, &t);
                },
            );
        }
        ViewMode::PreviewOnly => {
            ui.allocate_ui_with_layout(
                available_size,
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    render_preview_pane(ui, state, &t);
                },
            );
        }
        ViewMode::Split => {
            let pane_width = ((available_size.x - 1.0) / 2.0).max(100.0);
            let pane_height = available_size.y;

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(pane_width, pane_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        render_editor_pane(ui, state, &t);
                    },
                );

                ui.separator();

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), pane_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        render_preview_pane(ui, state, &t);
                    },
                );
            });
        }
    }
}

fn render_editor_pane(ui: &mut Ui, state: &mut AppState, t: &Tokens) {
    let is_dragged = egui::DragAndDrop::has_payload_of_type::<crate::state::DraggedNote>(ui.ctx());

    let frame = if is_dragged {
        Frame::NONE
            .fill(t.canvas)
            .stroke(Stroke::new(1.5, t.accent))
            .inner_margin(Margin::symmetric(28, 16))
    } else {
        Frame::NONE
            .fill(t.canvas)
            .inner_margin(Margin::symmetric(28, 16))
    };

    let frame_resp = frame.show(ui, |ui| {
        if is_dragged {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Drop note anywhere to insert [[link]]").strong().color(t.accent_text).size(12.0));
            });
            ui.add_space(6.0);
        }

        ScrollArea::vertical()
            .id_salt("editor_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Integrated Scheduled Events card if this is a daily note
                let maybe_daily_date = state.active_note.as_ref().and_then(|id| state.daily_config.date_of(id));
                if let Some(daily_date) = maybe_daily_date {
                    render_daily_events_widget(ui, state, daily_date, t);
                    ui.add_space(12.0);
                }

                let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
                    let mut layout_job = highlight_markdown(buf.as_str(), t);
                    layout_job.wrap.max_width = wrap_width;
                    ui.ctx().fonts_mut(|f| f.layout_job(layout_job))
                };

                let output = egui::TextEdit::multiline(&mut state.editor_buffer)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter)
                    .frame(Frame::NONE)
                    .show(ui);

                if let Some(cursor_range) = output.cursor_range {
                    let cursor_char_idx = cursor_range.primary.index.0;
                    let byte_offset = state
                        .editor_buffer
                        .char_indices()
                        .nth(cursor_char_idx)
                        .map(|(b, _)| b)
                        .unwrap_or(state.editor_buffer.len());
                    state.editor_cursor = byte_offset;
                }

                if output.response.changed() {
                    state.editor_dirty = true;
                }

                // Wiki-link autocomplete popup when typing `[[`
                render_link_autocomplete(ui, state, &output, t);

                ui.add_space(80.0);
            });
    });

    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
        if frame_resp.response.rect.contains(hover_pos) && ui.input(|i| i.pointer.any_released()) {
            if let Some(dragged) = egui::DragAndDrop::take_payload::<crate::state::DraggedNote>(ui.ctx()) {
                let link_str = format!("[[{}]]", dragged.name);
                if !state.editor_buffer.is_empty() && !state.editor_buffer.ends_with(' ') && !state.editor_buffer.ends_with('\n') {
                    state.editor_buffer.push(' ');
                }
                state.editor_buffer.push_str(&link_str);
                state.editor_buffer.push(' ');
                state.editor_dirty = true;
                state.set_status(format!("Linked note: {link_str}"));
            }
        }
    }
}

fn render_link_autocomplete(
    ui: &mut Ui,
    state: &mut AppState,
    output: &egui::text_edit::TextEditOutput,
    t: &Tokens,
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
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.card_border))
                    .corner_radius(CornerRadius::same(t.radius_sm as u8))
                    .inner_margin(Margin::same(8))
                    .shadow(egui::Shadow {
                        offset: [0, 6],
                        blur: 16,
                        spread: 0,
                        color: t.shadow,
                    })
                    .show(ui, |ui| {
                        theme::section_label(ui, t, "Link to note");
                        ui.add_space(2.0);
                        for note_name in matches {
                            if theme::list_row(ui, t, false, Some(t.accent), &format!("[[{note_name}]]"), None).clicked() {
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

fn render_preview_pane(ui: &mut Ui, state: &mut AppState, t: &Tokens) {
    let is_dragged = egui::DragAndDrop::has_payload_of_type::<crate::state::DraggedNote>(ui.ctx());

    let frame = if is_dragged {
        Frame::NONE
            .fill(t.canvas)
            .stroke(Stroke::new(1.5, t.accent))
            .inner_margin(Margin::symmetric(28, 16))
    } else {
        Frame::NONE
            .fill(t.canvas)
            .inner_margin(Margin::symmetric(28, 16))
    };

    let frame_resp = frame.show(ui, |ui| {
        if is_dragged {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Drop note anywhere to insert [[link]]").strong().color(t.accent_text).size(12.0));
            });
            ui.add_space(6.0);
        }

        ScrollArea::vertical()
            .id_salt("preview_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width(theme::READABLE_WIDTH);

                        // 1. Obsidian-style Frontmatter Properties
                        let (frontmatter, _) = Frontmatter::parse(&state.editor_buffer);
                        if !frontmatter.is_empty() {
                            render_properties_card(ui, &frontmatter, t);
                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(10.0);
                        }

                        // Integrated Scheduled Events card if this is a daily note
                        let maybe_daily_date = state.active_note.as_ref().and_then(|id| state.daily_config.date_of(id));
                        if let Some(daily_date) = maybe_daily_date {
                            render_daily_events_widget(ui, state, daily_date, t);
                            ui.add_space(12.0);
                        }

                        // 2. Note title if not explicitly started with H1
                        let blocks = parse_blocks(&state.editor_buffer);
                        let first_is_h1 = blocks.first().map_or(false, |b| matches!(b.block, Block::Heading { level: 1, .. }));
                        if !first_is_h1 {
                            if let Some(active_id) = &state.active_note {
                                ui.label(RichText::new(active_id.name()).size(26.0).strong().color(t.text));
                                ui.add_space(8.0);
                            }
                        }

                        let mut task_to_toggle: Option<usize> = None;
                        let mut note_to_open: Option<String> = None;
                        let mut date_to_open: Option<chrono::NaiveDate> = None;
                        let mut open_agenda = false;

                        for item in blocks {
                            let line_no = item.line;
                            match item.block {
                                Block::Heading { level, text } => {
                                    let (size, top_space, bottom_space) = match level {
                                        1 => (26.0, 18.0, 8.0),
                                        2 => (21.0, 14.0, 6.0),
                                        3 => (17.5, 10.0, 4.0),
                                        4 => (15.0, 8.0, 2.0),
                                        _ => (13.5, 6.0, 2.0),
                                    };
                                    ui.add_space(top_space);
                                    ui.label(RichText::new(text).size(size).color(t.text).strong());
                                    ui.add_space(bottom_space);
                                    if level == 1 {
                                        ui.separator();
                                        ui.add_space(4.0);
                                    }
                                }
                                Block::Paragraph(text) => {
                                    render_inline_spans(ui, text, &mut note_to_open, false, &state.vault, t);
                                    ui.add_space(6.0);
                                }
                                Block::ListItem { indent, marker, text } => {
                                    ui.horizontal(|ui| {
                                        if indent > 0 {
                                            ui.add_space(indent as f32 * 18.0);
                                        }
                                        let is_task_checked = match marker {
                                            ListMarker::Bullet => {
                                                ui.label(RichText::new("•").color(t.accent_text).size(13.0));
                                                false
                                            }
                                            ListMarker::Ordered(n) => {
                                                ui.label(RichText::new(format!("{n}.")).color(t.text_dim).size(12.0));
                                                false
                                            }
                                            ListMarker::Task { checked } => {
                                                let mut is_checked = checked;
                                                if ui.checkbox(&mut is_checked, "").changed() {
                                                    task_to_toggle = Some(line_no);
                                                }
                                                checked
                                            }
                                        };
                                        render_inline_spans(ui, text, &mut note_to_open, is_task_checked, &state.vault, t);
                                    });
                                    ui.add_space(2.0);
                                }
                                Block::Quote(text) => {
                                    render_quote_or_callout(ui, text, &mut note_to_open, &state.vault, t);
                                    ui.add_space(8.0);
                                }
                                Block::Code { lang, code } => {
                                    if lang.eq_ignore_ascii_case("event") {
                                        render_event_object_card(
                                            ui,
                                            state.active_note.as_ref(),
                                            &code,
                                            line_no,
                                            t,
                                            &mut date_to_open,
                                            &mut open_agenda,
                                        );
                                    } else if lang.eq_ignore_ascii_case("query") {
                                        render_query_object_card(
                                            ui,
                                            &state.vault,
                                            &state.event_store,
                                            &code,
                                            line_no,
                                            t,
                                            &mut note_to_open,
                                        );
                                    } else {
                                        Frame::NONE
                                            .fill(t.dock)
                                            .corner_radius(CornerRadius::same(t.radius_sm as u8))
                                            .inner_margin(Margin::same(12))
                                            .show(ui, |ui| {
                                                if !lang.is_empty() {
                                                    ui.horizontal(|ui| {
                                                        theme::pill(ui, &lang.to_uppercase(), t.text_faint, t.field);
                                                    });
                                                    ui.add_space(4.0);
                                                }
                                                ScrollArea::horizontal().id_salt(format!("code_block_{line_no}")).show(ui, |ui| {
                                                    ui.label(RichText::new(code.trim_end()).monospace().color(t.text).size(12.5));
                                                });
                                            });
                                        ui.add_space(8.0);
                                    }
                                }
                                Block::Rule => {
                                    ui.add_space(10.0);
                                    ui.separator();
                                    ui.add_space(10.0);
                                }
                                Block::Blank => {
                                    ui.add_space(6.0);
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

                        // If user clicked Daily Note in an event object
                        if let Some(date) = date_to_open {
                            state.open_daily_date(date);
                        }

                        // If user clicked Agenda in an event object
                        if open_agenda {
                            state.sidebar_section = crate::state::SidebarSection::Events;
                        }

                        ui.add_space(80.0);
                    });
                });
        });

    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
        if frame_resp.response.rect.contains(hover_pos) && ui.input(|i| i.pointer.any_released()) {
            if let Some(dragged) = egui::DragAndDrop::take_payload::<crate::state::DraggedNote>(ui.ctx()) {
                let link_str = format!("[[{}]]", dragged.name);
                if !state.editor_buffer.is_empty() && !state.editor_buffer.ends_with(' ') && !state.editor_buffer.ends_with('\n') {
                    state.editor_buffer.push(' ');
                }
                state.editor_buffer.push_str(&link_str);
                state.editor_buffer.push(' ');
                state.editor_dirty = true;
                state.set_status(format!("Linked note: {link_str}"));
            }
        }
    }
}

/// Integrated widget displaying scheduled events for a daily note with quick actions.
fn render_daily_events_widget(
    ui: &mut Ui,
    state: &mut AppState,
    date: chrono::NaiveDate,
    t: &Tokens,
) {
    let events = state.event_store.on(&state.vault, date);
    let card = theme::card_frame(t, false)
        .inner_margin(Margin::symmetric(14, 10));

    let mut event_to_open = None;
    let mut add_event_clicked = false;
    let mut sync_clicked = false;

    card.show(ui, |ui| {
        ui.horizontal(|ui| {
            theme::section_label(ui, t, &format!("Scheduled Events ({})", events.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::ghost_button(ui, t, RichText::new("+ Add Event").color(t.success).size(11.5)).clicked() {
                    add_event_clicked = true;
                }
                if !events.is_empty() {
                    if theme::ghost_button(ui, t, RichText::new("Sync into Note").color(t.accent_text).size(11.5))
                        .on_hover_text("Insert event checkboxes into note text")
                        .clicked()
                    {
                        sync_clicked = true;
                    }
                }
            });
        });

        ui.add_space(4.0);

        if events.is_empty() {
            ui.label(RichText::new("No events scheduled for this day.").color(t.text_faint).size(11.5).italics());
        } else {
            for ev in &events {
                ui.horizontal(|ui| {
                    theme::pill(ui, &ev.time_label(), t.warning, t.accent_soft);
                    ui.label(RichText::new(&ev.title).strong().size(12.5).color(t.text));
                    if let Some(loc) = &ev.location {
                        ui.label(RichText::new(format!("• {loc}")).size(11.5).color(t.text_dim));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::ghost_button(ui, t, RichText::new("Open Note →").color(t.accent_text)).clicked() {
                            event_to_open = Some(ev.id.clone());
                        }
                    });
                });
                ui.add_space(2.0);
            }
        }
    });

    if let Some(id) = event_to_open {
        state.open_note(id);
    }
    if add_event_clicked {
        state.new_event_draft = mybrain_core::EventDraft::new(date);
        state.show_new_event_dialog = true;
    }
    if sync_clicked {
        state.sync_events_to_daily_note(date);
    }
}

/// Renders an Obsidian 1.4+ style collapsible Properties / Metadata block
fn render_properties_card(ui: &mut Ui, frontmatter: &Frontmatter, t: &Tokens) {
    let card = theme::card_frame(t, false)
        .inner_margin(Margin::symmetric(14, 10));

    card.show(ui, |ui| {
        ui.horizontal(|ui| {
            theme::section_label(ui, t, "Properties");
        });
        ui.add_space(6.0);

        for (key, val) in frontmatter.iter() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(key).color(t.text_dim).size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if key == "tags" {
                        for tag in val.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                            theme::pill(ui, &format!("#{tag}"), t.accent_text, t.accent_soft);
                        }
                    } else {
                        ui.label(RichText::new(val).color(t.text).size(12.0));
                    }
                });
            });
            ui.add_space(2.0);
        }
    });
}

/// Renders either an Obsidian callout (`> [!NOTE]`) or a standard blockquote
fn render_quote_or_callout(
    ui: &mut Ui,
    text: &str,
    note_to_open: &mut Option<String>,
    vault: &Vault,
    t: &Tokens,
) {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("[!") {
        if let Some((kind_str, after)) = rest.split_once(']') {
            let kind = kind_str.trim().to_uppercase();
            let (accent_color, label) = match kind.as_str() {
                "NOTE" | "INFO" => (t.accent, "NOTE"),
                "TIP" | "HINT" | "SUCCESS" | "IMPORTANT" => (t.success, "TIP"),
                "WARNING" | "CAUTION" | "ATTENTION" => (t.warning, "WARNING"),
                "DANGER" | "ERROR" | "BUG" | "FAILURE" => (t.danger, "DANGER"),
                _ => (t.accent_text, kind.as_str()),
            };

            let body = after.trim();

            Frame::NONE
                .fill(t.accent_soft)
                .stroke(Stroke::new(1.0, t.accent_border))
                .corner_radius(CornerRadius::same(t.radius_sm as u8))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::pill(ui, label, accent_color, t.card);
                    });
                    ui.add_space(4.0);
                    render_inline_spans(ui, body, note_to_open, false, vault, t);
                });
            return;
        }
    }

    // Standard Obsidian blockquote with accent left bar
    Frame::NONE
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("▎").strong().color(t.accent).size(13.0));
                render_inline_spans(ui, text, note_to_open, false, vault, t);
            });
        });
}

fn render_inline_spans(
    ui: &mut Ui,
    text: &str,
    note_to_open: &mut Option<String>,
    is_task_checked: bool,
    vault: &Vault,
    t: &Tokens,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let spans = parse_inline(text);
        for span in spans {
            match span {
                Inline::Text(txt) => {
                    let mut rt = RichText::new(txt);
                    if is_task_checked {
                        rt = rt.color(t.text_faint).strikethrough();
                    } else {
                        rt = rt.color(t.text);
                    }
                    ui.label(rt);
                }
                Inline::Bold(txt) => {
                    let mut rt = RichText::new(txt).strong();
                    if is_task_checked {
                        rt = rt.color(t.text_faint).strikethrough();
                    } else {
                        rt = rt.color(t.text);
                    }
                    ui.label(rt);
                }
                Inline::Italic(txt) => {
                    let mut rt = RichText::new(txt).italics();
                    if is_task_checked {
                        rt = rt.color(t.text_faint).strikethrough();
                    } else {
                        rt = rt.color(t.text_dim);
                    }
                    ui.label(rt);
                }
                Inline::Code(code) => {
                    Frame::NONE
                        .fill(t.dock)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(5, 1))
                        .show(ui, |ui| {
                            ui.label(RichText::new(code).monospace().color(t.accent_text).size(12.0));
                        });
                }
                Inline::Url(u) => {
                    ui.hyperlink_to(RichText::new(u).color(t.accent).underline(), u);
                }
                Inline::Tag(tag) => {
                    theme::pill(ui, &format!("#{tag}"), t.accent_text, t.accent_soft);
                }
                Inline::Link(link) => {
                    let label = link.display_text();
                    let is_resolved = vault.resolve_link(&link.target).is_some();
                    let link_color = if is_resolved {
                        t.accent_text
                    } else {
                        t.warning
                    };

                    let btn = ui.add(
                        egui::Button::new(
                            RichText::new(label)
                                .color(link_color)
                                .underline(),
                        )
                        .frame(false),
                    );

                    let hover_text = if is_resolved {
                        format!("Open note: {}", link.target)
                    } else {
                        format!("Create note: {} (uncreated)", link.target)
                    };

                    if btn.on_hover_text(hover_text).clicked() {
                        *note_to_open = Some(link.target.clone());
                    }
                }
                Inline::Image { alt, url } => {
                    Frame::NONE
                        .fill(t.dock)
                        .stroke(Stroke::new(1.0, t.field_border))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                theme::icon_image(ui, 12.0, t.accent_text);
                                let display = if !alt.is_empty() { alt } else { url };
                                ui.label(RichText::new(display).color(t.accent_text).size(12.0).italics());
                            });
                        })
                        .response
                        .on_hover_text(format!("Image: {url}"));
                }
            }
        }
    });
}

fn highlight_markdown(text: &str, t: &Tokens) -> LayoutJob {
    let mut job = LayoutJob::default();
    let default_font = FontId::monospace(13.0);
    let heading_font = FontId::monospace(14.5);

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            job.append(
                line,
                0.0,
                TextFormat {
                    font_id: heading_font.clone(),
                    color: t.accent_text,
                    ..Default::default()
                },
            );
        } else if trimmed.starts_with("```") {
            job.append(
                line,
                0.0,
                TextFormat {
                    font_id: default_font.clone(),
                    color: t.warning,
                    ..Default::default()
                },
            );
        } else if trimmed.starts_with('>') {
            job.append(
                line,
                0.0,
                TextFormat {
                    font_id: default_font.clone(),
                    color: t.accent,
                    ..Default::default()
                },
            );
        } else {
            // Highlight [[wiki links]], tags, and text within the line
            let mut last = 0;
            for link in mybrain_core::links::parse_links(line) {
                if link.range.start > last {
                    job.append(
                        &line[last..link.range.start],
                        0.0,
                        TextFormat {
                            font_id: default_font.clone(),
                            color: t.text,
                            ..Default::default()
                        },
                    );
                }
                job.append(
                    &line[link.range.clone()],
                    0.0,
                    TextFormat {
                        font_id: default_font.clone(),
                        color: t.accent_text,
                        underline: Stroke::new(1.0, t.accent_border),
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
                        color: t.text,
                        ..Default::default()
                    },
                );
            }
        }
    }
    job
}

/// Renders an embedded ```event ... ``` object card in live preview.
fn render_event_object_card(
    ui: &mut Ui,
    active_note: Option<&NoteId>,
    code: &str,
    line_no: usize,
    t: &Tokens,
    open_daily: &mut Option<chrono::NaiveDate>,
    open_agenda: &mut bool,
) {
    let dummy_id = active_note.cloned().unwrap_or_else(|| NoteId::new("Note").unwrap());
    let dummy_note = mybrain_core::Note::new(dummy_id, "");
    let lines: Vec<&str> = code.lines().collect();
    let ev = mybrain_core::Event::parse_block(&dummy_note, line_no, &lines);

    let Some(ev) = ev else {
        Frame::NONE
            .fill(t.dock)
            .corner_radius(CornerRadius::same(t.radius_sm as u8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.label(RichText::new(code).monospace().color(t.text_dim).size(12.0));
            });
        return;
    };

    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.5, t.warning))
        .corner_radius(CornerRadius::same(t.radius as u8))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::pill(ui, "EVENT", Color32::WHITE, t.warning);
                ui.label(RichText::new(&ev.title).size(14.0).strong().color(t.text));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::ghost_button(ui, t, RichText::new("Agenda ↗").color(t.warning).size(11.5)).clicked() {
                        *open_agenda = true;
                    }
                    if theme::ghost_button(ui, t, RichText::new("Daily Note ↗").color(t.accent_text).size(11.5)).clicked() {
                        *open_daily = Some(ev.date);
                    }
                });
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                theme::pill(ui, &ev.date.format("%a, %b %-d, %Y").to_string(), t.accent_text, t.accent_soft);
                theme::pill(ui, &ev.time_label(), t.warning, t.accent_soft);
                if let Some(loc) = &ev.location {
                    theme::icon_pin(ui, 12.0, t.text_dim);
                    ui.label(RichText::new(loc).size(12.0).color(t.text_dim));
                }
            });

            if let Some(desc) = &ev.description {
                if !desc.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(desc).size(12.0).color(t.text_dim).italics());
                }
            }
        });

    ui.add_space(8.0);
}

/// Renders a dynamic ```query ... ``` object card in live preview.
fn render_query_object_card(
    ui: &mut Ui,
    vault: &Vault,
    event_store: &mybrain_core::EventStore,
    code: &str,
    _line_no: usize,
    t: &Tokens,
    note_to_open: &mut Option<String>,
) {
    let query_trim = code.trim();
    let query_lower = query_trim.to_lowercase();

    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.2, t.accent_border))
        .corner_radius(CornerRadius::same(t.radius as u8))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::pill(ui, "QUERY", t.accent_text, t.accent_soft);
                ui.label(RichText::new(query_trim).monospace().size(12.5).color(t.text));
            });

            ui.add_space(8.0);

            // Execute query against vault / events
            if query_lower == "events" || query_lower.starts_with("events:") {
                let all_events = event_store.all(vault);
                if all_events.is_empty() {
                    ui.label(RichText::new("No events found across vault.").color(t.text_faint).size(12.0).italics());
                } else {
                    for ev in all_events.iter().take(15) {
                        ui.horizontal(|ui| {
                            theme::pill(ui, &ev.date.format("%b %-d").to_string(), t.accent_text, t.accent_soft);
                            theme::pill(ui, &ev.time_label(), t.warning, t.accent_soft);
                            ui.label(RichText::new(&ev.title).strong().size(12.0).color(t.text));
                            ui.label(RichText::new(format!("in [[{}]]", ev.id.name())).size(11.0).color(t.text_dim));
                            if ui.link(RichText::new("Open →").size(11.0).color(t.accent_text)).clicked() {
                                *note_to_open = Some(ev.id.name().to_string());
                            }
                        });
                        ui.add_space(2.0);
                    }
                }
            } else if query_lower == "todo" || query_lower == "tasks" {
                let mut matches = Vec::new();
                for note in vault.notes() {
                    for line in note.content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("- [ ]") {
                            let task_text = trimmed.trim_start_matches("- [ ]").trim();
                            matches.push((note.id.name().to_string(), task_text.to_string()));
                            if matches.len() >= 20 {
                                break;
                            }
                        }
                    }
                    if matches.len() >= 20 {
                        break;
                    }
                }
                if matches.is_empty() {
                    ui.label(RichText::new("No open tasks found across notes.").color(t.text_faint).size(12.0).italics());
                } else {
                    for (note_name, task_desc) in matches {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("☐").color(t.text_dim).size(12.0));
                            ui.label(RichText::new(task_desc).size(12.0).color(t.text));
                            ui.label(RichText::new(format!("in [[{}]]", note_name)).size(11.0).color(t.text_dim));
                            if ui.link(RichText::new("Open →").size(11.0).color(t.accent_text)).clicked() {
                                *note_to_open = Some(note_name);
                            }
                        });
                        ui.add_space(2.0);
                    }
                }
            } else if let Some(tag_filter) = query_trim.strip_prefix("tag:") {
                let clean_tag = tag_filter.trim().trim_start_matches('#');
                let tag_needle = format!("#{clean_tag}");
                let mut matched_notes = Vec::new();
                for note in vault.notes() {
                    let has_in_frontmatter = note.frontmatter().get("tags")
                        .map(|tags| tags.split(',').any(|t| t.trim().eq_ignore_ascii_case(clean_tag)))
                        .unwrap_or(false);
                    let has_in_content = note.content.contains(&tag_needle);
                    if has_in_frontmatter || has_in_content {
                        matched_notes.push(note.id.name().to_string());
                    }
                }
                if matched_notes.is_empty() {
                    ui.label(RichText::new(format!("No notes tagged #{clean_tag}.")).color(t.text_faint).size(12.0).italics());
                } else {
                    for name in matched_notes {
                        ui.horizontal(|ui| {
                            theme::pill(ui, &format!("#{clean_tag}"), t.accent_text, t.accent_soft);
                            if ui.link(RichText::new(format!("[[{name}]]")).size(12.5).color(t.accent_text)).clicked() {
                                *note_to_open = Some(name);
                            }
                        });
                        ui.add_space(2.0);
                    }
                }
            } else if let Some(folder_filter) = query_trim.strip_prefix("folder:") {
                let folder = folder_filter.trim();
                let matched_notes: Vec<_> = vault.notes()
                    .filter(|n| n.id.folder().map(|f| f.eq_ignore_ascii_case(folder)).unwrap_or(false))
                    .map(|n| n.id.name().to_string())
                    .collect();
                if matched_notes.is_empty() {
                    ui.label(RichText::new(format!("No notes found in folder '{folder}'.")).color(t.text_faint).size(12.0).italics());
                } else {
                    for name in matched_notes {
                        ui.horizontal(|ui| {
                            theme::pill(ui, folder, t.text_faint, t.field);
                            if ui.link(RichText::new(format!("[[{name}]]")).size(12.5).color(t.accent_text)).clicked() {
                                *note_to_open = Some(name);
                            }
                        });
                        ui.add_space(2.0);
                    }
                }
            } else {
                let search_term = query_trim.strip_prefix("search:").unwrap_or(query_trim).trim();
                let results = vault.search(search_term);
                if results.is_empty() {
                    ui.label(RichText::new(format!("No notes matching '{search_term}'.")).color(t.text_faint).size(12.0).italics());
                } else {
                    for hit in results.iter().take(10) {
                        ui.vertical(|ui| {
                            if ui.link(RichText::new(format!("[[{}]]", hit.id.name())).strong().size(12.5).color(t.accent_text)).clicked() {
                                *note_to_open = Some(hit.id.name().to_string());
                            }
                            if let Some(snippet) = &hit.snippet {
                                ui.label(RichText::new(snippet).color(t.text_dim).size(11.0));
                            }
                        });
                        ui.add_space(3.0);
                    }
                }
            }
        });
    ui.add_space(8.0);
}
