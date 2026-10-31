//! Markdown editor and live preview plugin for Brainwash.

use egui::{
    Color32, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui,
};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::frontmatter::Frontmatter;
use mybrain_core::index::LinkIndex;
use mybrain_core::markdown::{
    parse_blocks, parse_inline, toggle_task, Block, Inline, ListMarker,
};
use mybrain_core::note::NoteId;
use mybrain_core::vault::Vault;

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::{self, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewMode {
    PreviewOnly,
    EditOnly,
    Split,
}

pub struct MarkdownPlugin {
    pub active_note: Option<NoteId>,
    pub open_tabs: Vec<NoteId>,
    pub editor_buffer: String,
    pub editor_cursor: usize,
    pub editor_selection: Option<(usize, usize)>,
    pub editor_dirty: bool,
    pub view_mode: ViewMode,
    pub history: Vec<NoteId>,
    pub history_idx: usize,
}

impl MarkdownPlugin {
    pub fn new() -> Self {
        Self {
            active_note: None,
            open_tabs: Vec::new(),
            editor_buffer: String::new(),
            editor_cursor: 0,
            editor_selection: None,
            editor_dirty: false,
            view_mode: ViewMode::PreviewOnly,
            history: Vec::new(),
            history_idx: 0,
        }
    }

    pub fn save_current(&mut self, vault: &mut Vault, index: &mut LinkIndex) {
        if let Some(ref id) = self.active_note {
            if self.editor_dirty {
                if let Err(e) = vault.save(id, &self.editor_buffer) {
                    eprintln!("Failed to save note: {e}");
                } else {
                    *index = LinkIndex::build(vault);
                    self.editor_dirty = false;
                }
            }
        }
    }

    pub fn open_note(&mut self, id: NoteId, vault: &mut Vault, index: &mut LinkIndex) {
        if self.active_note.as_ref() == Some(&id) {
            return;
        }

        self.save_current(vault, index);

        if let Some(note) = vault.get(&id) {
            self.active_note = Some(id.clone());
            self.editor_buffer = note.content.clone();
            self.editor_cursor = 0;
            self.editor_selection = None;
            self.editor_dirty = false;

            if !self.open_tabs.contains(&id) {
                self.open_tabs.push(id.clone());
            }

            if self.history.get(self.history_idx) != Some(&id) {
                self.history.truncate(self.history_idx + 1);
                self.history.push(id);
                self.history_idx = self.history.len().saturating_sub(1);
            }
        }
    }

    pub fn close_tab(&mut self, id: &NoteId, vault: &mut Vault, index: &mut LinkIndex) {
        self.save_current(vault, index);
        if let Some(pos) = self.open_tabs.iter().position(|t| t == id) {
            self.open_tabs.remove(pos);
            if self.active_note.as_ref() == Some(id) {
                if self.open_tabs.is_empty() {
                    self.active_note = None;
                    self.editor_buffer.clear();
                    self.editor_dirty = false;
                } else {
                    let next_pos = pos.min(self.open_tabs.len() - 1);
                    let next_id = self.open_tabs[next_pos].clone();
                    self.open_note(next_id, vault, index);
                }
            }
        }
    }

    pub fn close_current_tab(&mut self, vault: &mut Vault, index: &mut LinkIndex) {
        if let Some(active) = self.active_note.clone() {
            self.close_tab(&active, vault, index);
        }
    }

    pub fn cycle_next_tab(&mut self, vault: &mut Vault, index: &mut LinkIndex) {
        if self.open_tabs.len() <= 1 {
            return;
        }
        if let Some(active) = &self.active_note {
            if let Some(pos) = self.open_tabs.iter().position(|t| t == active) {
                let next_pos = (pos + 1) % self.open_tabs.len();
                let next_id = self.open_tabs[next_pos].clone();
                self.open_note(next_id, vault, index);
            }
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.history_idx > 0 && !self.history.is_empty()
    }

    pub fn go_back(&mut self, vault: &mut Vault, index: &mut LinkIndex) {
        if self.can_go_back() {
            self.save_current(vault, index);
            self.history_idx -= 1;
            let target = self.history[self.history_idx].clone();
            if let Some(note) = vault.get(&target) {
                self.active_note = Some(target.clone());
                self.editor_buffer = note.content.clone();
                self.editor_dirty = false;
                if !self.open_tabs.contains(&target) {
                    self.open_tabs.push(target);
                }
            }
        }
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_idx + 1 < self.history.len()
    }

    pub fn go_forward(&mut self, vault: &mut Vault, index: &mut LinkIndex) {
        if self.can_go_forward() {
            self.save_current(vault, index);
            self.history_idx += 1;
            let target = self.history[self.history_idx].clone();
            if let Some(note) = vault.get(&target) {
                self.active_note = Some(target.clone());
                self.editor_buffer = note.content.clone();
                self.editor_dirty = false;
                if !self.open_tabs.contains(&target) {
                    self.open_tabs.push(target);
                }
            }
        }
    }

    pub fn apply_inline_format(&mut self, prefix: &str, suffix: &str, placeholder: &str) {
        if self.active_note.is_none() {
            return;
        }
        if let Some((start, end)) = self.editor_selection {
            if start < end
                && end <= self.editor_buffer.len()
                && self.editor_buffer.is_char_boundary(start)
                && self.editor_buffer.is_char_boundary(end)
            {
                let selected = &self.editor_buffer[start..end];
                let replacement = format!("{prefix}{selected}{suffix}");
                let mut new_buf =
                    String::with_capacity(self.editor_buffer.len() + prefix.len() + suffix.len());
                new_buf.push_str(&self.editor_buffer[..start]);
                new_buf.push_str(&replacement);
                new_buf.push_str(&self.editor_buffer[end..]);
                self.editor_buffer = new_buf;
                self.editor_dirty = true;
                let new_cursor = start + replacement.len();
                self.editor_cursor = new_cursor;
                self.editor_selection = None;
                return;
            }
        }
        let snippet = format!("{prefix}{placeholder}{suffix}");
        self.insert_snippet_into_active_note(&snippet);
    }

    pub fn insert_snippet_into_active_note(&mut self, snippet: &str) {
        if self.active_note.is_none() {
            return;
        }
        let cursor = self.editor_cursor.min(self.editor_buffer.len());
        let (left, right) = self.editor_buffer.split_at(cursor);
        let mut new_buf = String::with_capacity(self.editor_buffer.len() + snippet.len() + 2);
        new_buf.push_str(left);
        if !left.is_empty() && !left.ends_with('\n') && (snippet.starts_with("```") || snippet.starts_with('#') || snippet.starts_with("- ")) {
            new_buf.push('\n');
        }
        new_buf.push_str(snippet);
        new_buf.push_str(right);
        self.editor_buffer = new_buf;
        self.editor_dirty = true;
        self.editor_cursor = cursor + snippet.len();
    }

    pub fn insert_table(&mut self, cols: usize, rows: usize) {
        let cols = cols.clamp(1, 12);
        let rows = rows.clamp(1, 50);
        let mut out = String::from("\n| ");
        for c in 1..=cols {
            out.push_str(&format!("Header {c} | "));
        }
        out.push('\n');
        out.push_str("| ");
        for _ in 1..=cols {
            out.push_str("--- | ");
        }
        out.push('\n');
        for r in 1..=rows {
            out.push_str("| ");
            for c in 1..=cols {
                out.push_str(&format!("Cell {r}-{c} | "));
            }
            out.push('\n');
        }
        out.push('\n');
        self.insert_snippet_into_active_note(&out);
    }

    pub fn insert_image(&mut self, alt: &str, url: &str) {
        let snippet = format!("![{alt}]({url})");
        self.insert_snippet_into_active_note(&snippet);
    }
}

impl Default for MarkdownPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for MarkdownPlugin {
    fn id(&self) -> &'static str {
        "core.markdown"
    }

    fn name(&self) -> &'static str {
        "Markdown Notes"
    }

    fn description(&self) -> &'static str {
        "Markdown editor and live preview with wiki-links, task toggles, syntax highlighting, and tabs"
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "📝",
            tooltip: "Markdown Notes (Ctrl+1)",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::MainWorkspace,
            priority: 20,
        })
    }

    fn on_disable(&mut self, ctx: &mut PluginContext) {
        if self.editor_dirty {
            self.save_current(ctx.vault, ctx.index);
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "save_note".into(),
                title: "Save Current Note".into(),
                shortcut: Some("Ctrl+S".into()),
                plugin_id: "core.markdown",
            },
            PluginCommand {
                id: "toggle_view_mode".into(),
                title: "Toggle Edit / Preview Mode".into(),
                shortcut: Some("Ctrl+E".into()),
                plugin_id: "core.markdown",
            },
            PluginCommand {
                id: "close_tab".into(),
                title: "Close Current Tab".into(),
                shortcut: Some("Ctrl+W".into()),
                plugin_id: "core.markdown",
            },
            PluginCommand {
                id: "cycle_tab".into(),
                title: "Next Tab".into(),
                shortcut: Some("Ctrl+Tab".into()),
                plugin_id: "core.markdown",
            },
            PluginCommand {
                id: "open_markdown".into(),
                title: "Open Markdown Notes View".into(),
                shortcut: Some("Ctrl+1".into()),
                plugin_id: "core.markdown",
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        match command_id {
            "save_note" => {
                self.save_current(ctx.vault, ctx.index);
                ctx.toast("Note saved");
            }
            "toggle_view_mode" => {
                self.view_mode = match self.view_mode {
                    ViewMode::PreviewOnly => ViewMode::EditOnly,
                    ViewMode::EditOnly => ViewMode::PreviewOnly,
                    ViewMode::Split => ViewMode::PreviewOnly,
                };
            }
            "close_tab" => {
                self.close_current_tab(ctx.vault, ctx.index);
            }
            "cycle_tab" => {
                self.cycle_next_tab(ctx.vault, ctx.index);
            }
            "open_markdown" => {
                ctx.activate_plugin("core.markdown");
            }
            _ => {}
        }
    }

    fn on_event(&mut self, event: &AppEvent, ctx: &mut PluginContext) {
        match event {
            AppEvent::RequestOpenNote(id) => {
                self.open_note(id.clone(), ctx.vault, ctx.index);
                ctx.activate_plugin("core.markdown");
            }
            AppEvent::NoteSaved(id) => {
                if self.active_note.as_ref() == Some(id) {
                    self.editor_dirty = false;
                }
            }
            AppEvent::NoteDeleted(id) => {
                self.close_tab(id, ctx.vault, ctx.index);
            }
            AppEvent::NoteRenamed { old, new } => {
                if let Some(pos) = self.open_tabs.iter().position(|t| t == old) {
                    self.open_tabs[pos] = new.clone();
                }
                if self.active_note.as_ref() == Some(old) {
                    self.active_note = Some(new.clone());
                }
                for h in &mut self.history {
                    if h == old {
                        *h = new.clone();
                    }
                }
            }
            AppEvent::InsertText(text) => {
                self.insert_snippet_into_active_note(text);
            }
            _ => {}
        }
    }

    fn render_main_view(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());
        let panel_max_rect = ui.max_rect();
        ui.set_max_width(panel_max_rect.width());
        ui.set_clip_rect(panel_max_rect.intersect(ui.clip_rect()));

        // 1. Tab Bar
        self.render_tab_bar(ui, ctx, &t);
        theme::bottom_hairline(ui, ui.max_rect(), &t);

        let Some(active_id) = self.active_note.clone() else {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No note selected").color(t.text_dim).size(17.0).strong());
                    ui.add_space(8.0);
                    ui.label(RichText::new("Choose a note from the File Explorer or press Ctrl+N to create one.").color(t.text_faint).size(13.0));
                });
            });
            return;
        };

        // 2. Note Header & Toolbar
        Frame::NONE
            .fill(t.canvas)
            .inner_margin(Margin::symmetric(24, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let can_back = self.can_go_back();
                    let can_fwd = self.can_go_forward();
                    if ui.add_enabled(can_back, egui::Button::new("←").frame(false)).clicked() {
                        self.go_back(ctx.vault, ctx.index);
                    }
                    if ui.add_enabled(can_fwd, egui::Button::new("→").frame(false)).clicked() {
                        self.go_forward(ctx.vault, ctx.index);
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Note Breadcrumb & Title
                    if let Some(folder) = active_id.folder() {
                        let folder_emoji = ctx.settings.folder_emojis.get(folder).cloned().unwrap_or_else(|| "📁".to_string());
                        theme::pill(ui, &format!("{folder_emoji} {folder}"), t.text_dim, t.field);
                        ui.label(RichText::new("/").color(t.text_faint).size(12.0));
                    }

                    let note = ctx.vault.get(&active_id);
                    let note_fm = note.map(|n| n.frontmatter());
                    let note_color = note_fm.as_ref().and_then(|fm| fm.get("color")).and_then(crate::theme::parse_color_str);
                    let note_emoji = note_fm.as_ref().and_then(|fm| fm.get("icon")).map(|s| s.to_string());
                    let is_daily = active_id.as_str().starts_with("Daily/");
                    let is_event = active_id.as_str().starts_with("Events/");

                    let default_icon = if is_daily { "📅" } else if is_event { "🔔" } else { "📝" };
                    let current_icon = note_emoji.as_deref().unwrap_or(default_icon);
                    let icon_color = note_color.unwrap_or(t.accent);

                    ui.label(RichText::new(current_icon).size(15.0).color(icon_color));
                    ui.label(RichText::new(active_id.name()).color(t.text).strong().size(15.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // View mode segmented toggle
                        let mode = self.view_mode;
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
                            self.view_mode = new_mode;
                        }

                        ui.separator();

                        // Save button
                        if self.editor_dirty {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Save (Ctrl+S)")
                                            .color(Color32::WHITE)
                                            .size(11.5)
                                            .strong(),
                                    )
                                    .fill(t.accent)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                                )
                                .clicked()
                            {
                                self.save_current(ctx.vault, ctx.index);
                                ctx.toast("Note saved");
                            }
                        }
                    });
                });
            });

        theme::bottom_hairline(ui, ui.max_rect(), &t);

        // 3. Formatting Toolbar
        self.render_formatting_toolbar(ui, &t);
        theme::bottom_hairline(ui, ui.max_rect(), &t);

        // 4. Main Workspace Area
        let workspace_rect = egui::Rect::from_min_max(ui.cursor().min, panel_max_rect.max);
        ui.set_max_width(workspace_rect.width());
        let available_size = workspace_rect.size().max(egui::Vec2::ZERO);

        match self.view_mode {
            ViewMode::EditOnly => {
                ui.allocate_ui_with_layout(
                    available_size,
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_size(available_size);
                        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                        self.render_editor_pane(ui, ctx, &t);
                    },
                );
            }
            ViewMode::PreviewOnly => {
                ui.allocate_ui_with_layout(
                    available_size,
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_size(available_size);
                        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                        self.render_preview_pane(ui, ctx, &t);
                    },
                );
            }
            ViewMode::Split => {
                let sep_space = 1.0;
                let pane_width = ((available_size.x - sep_space) / 2.0).max(50.0);
                let pane_height = available_size.y;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.set_max_size(available_size);

                    ui.allocate_ui_with_layout(
                        egui::vec2(pane_width, pane_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_max_size(egui::vec2(pane_width, pane_height));
                            ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                            self.render_editor_pane(ui, ctx, &t);
                        },
                    );

                    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(sep_space, pane_height), egui::Sense::hover());
                    ui.painter().vline(sep_rect.center().x, sep_rect.y_range(), Stroke::new(1.0, t.separator));

                    let right_width = (available_size.x - pane_width - sep_space).max(50.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(right_width, pane_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_max_size(egui::vec2(right_width, pane_height));
                            ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
                            self.render_preview_pane(ui, ctx, &t);
                        },
                    );
                });
            }
        }
    }
}

impl MarkdownPlugin {
    fn render_tab_bar(&mut self, ui: &mut Ui, ctx: &mut PluginContext, t: &Tokens) {
        if self.open_tabs.is_empty() {
            return;
        }

        let mut tab_to_close = None;
        let mut tab_to_select = None;

        Frame::NONE
            .fill(t.dock)
            .inner_margin(Margin::symmetric(8, 0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    for id in &self.open_tabs {
                        let is_active = self.active_note.as_ref() == Some(id);
                        let bg = if is_active { t.canvas } else { egui::Color32::TRANSPARENT };
                        let fg = if is_active { t.text } else { t.text_faint };

                        Frame::NONE
                            .fill(bg)
                            .corner_radius(CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 })
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let btn = egui::Button::new(RichText::new(id.name()).size(12.5).color(fg))
                                        .frame(false);
                                    if ui.add(btn).clicked() {
                                        tab_to_select = Some(id.clone());
                                    }

                                    if theme::ghost_button(ui, t, RichText::new("×").size(12.0).color(t.text_faint)).clicked() {
                                        tab_to_close = Some(id.clone());
                                    }
                                });
                            });
                    }
                });
            });

        if let Some(id) = tab_to_select {
            self.open_note(id, ctx.vault, ctx.index);
        }

        if let Some(id) = tab_to_close {
            self.close_tab(&id, ctx.vault, ctx.index);
        }
    }

    fn render_formatting_toolbar(&mut self, ui: &mut Ui, t: &Tokens) {
        Frame::NONE
            .fill(t.dock)
            .inner_margin(Margin::symmetric(16, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if theme::ghost_button(ui, t, RichText::new("B").strong().color(t.text)).clicked() {
                        self.apply_inline_format("**", "**", "bold");
                    }
                    if theme::ghost_button(ui, t, RichText::new("I").italics().color(t.text)).clicked() {
                        self.apply_inline_format("*", "*", "italic");
                    }
                    if theme::ghost_button(ui, t, RichText::new("Code").monospace().color(t.text)).clicked() {
                        self.apply_inline_format("`", "`", "code");
                    }
                    ui.separator();
                    if theme::ghost_button(ui, t, RichText::new("[[Link]]").color(t.accent_text)).clicked() {
                        self.apply_inline_format("[[", "]]", "Note");
                    }
                    if theme::ghost_button(ui, t, RichText::new("Task").color(t.text)).clicked() {
                        self.apply_inline_format("- [ ] ", "", "Task item");
                    }
                    if theme::ghost_button(ui, t, RichText::new("Table").color(t.text)).clicked() {
                        self.insert_table(3, 2);
                    }
                    if theme::ghost_button(ui, t, RichText::new("Image").color(t.text)).clicked() {
                        self.insert_image("Alt text", "https://");
                    }
                });
            });
    }

    fn render_editor_pane(&mut self, ui: &mut Ui, _ctx: &mut PluginContext, t: &Tokens) {
        Frame::NONE
            .fill(t.canvas)
            .inner_margin(Margin::symmetric(24, 16))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("editor_scroll_area")
                    .show(ui, |ui| {
                        let edit = egui::TextEdit::multiline(&mut self.editor_buffer)
                            .desired_width(ui.available_width())
                            .font(egui::TextStyle::Monospace)
                            .frame(Frame::NONE);

                        let resp = ui.add(edit);
                        if resp.changed() {
                            self.editor_dirty = true;
                        }
                    });
            });
    }

    fn render_preview_pane(&mut self, ui: &mut Ui, ctx: &mut PluginContext, t: &Tokens) {
        Frame::NONE
            .fill(t.canvas)
            .inner_margin(Margin::symmetric(28, 16))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("preview_scroll_area")
                    .show(ui, |ui| {
                        let (frontmatter, _) = Frontmatter::parse(&self.editor_buffer);
                        if !frontmatter.is_empty() {
                            Frame::NONE
                                .fill(t.dock)
                                .corner_radius(CornerRadius::same(t.radius_sm as u8))
                                .inner_margin(Margin::same(10))
                                .show(ui, |ui| {
                                    for (k, v) in frontmatter.iter() {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(k).strong().size(11.5).color(t.text_faint));
                                            ui.label(RichText::new(v).size(12.0).color(t.text));
                                        });
                                    }
                                });
                            ui.add_space(10.0);
                        }

                        let blocks = parse_blocks(&self.editor_buffer);
                        let mut task_toggle = None;
                        let mut link_clicked = None;

                        for item in blocks {
                            match item.block {
                                Block::Heading { level, text } => {
                                    let size = match level {
                                        1 => 24.0,
                                        2 => 20.0,
                                        3 => 16.5,
                                        _ => 14.0,
                                    };
                                    ui.add_space(8.0);
                                    ui.label(RichText::new(text).size(size).strong().color(t.text));
                                    ui.add_space(4.0);
                                }
                                Block::Paragraph(text) => {
                                    render_inline_spans(ui, text, &mut link_clicked, false, ctx.vault, t);
                                    ui.add_space(6.0);
                                }
                                Block::ListItem { indent, marker, text } => {
                                    ui.horizontal_top(|ui| {
                                        if indent > 0 {
                                            ui.add_space(indent as f32 * 16.0);
                                        }
                                        let is_checked = match marker {
                                            ListMarker::Bullet => {
                                                ui.label(RichText::new("•").color(t.accent_text).size(13.0));
                                                false
                                            }
                                            ListMarker::Ordered(n) => {
                                                ui.label(RichText::new(format!("{n}.")).color(t.text_dim).size(12.0));
                                                false
                                            }
                                            ListMarker::Task { checked } => {
                                                let mut c = checked;
                                                if ui.checkbox(&mut c, "").changed() {
                                                    task_toggle = Some(item.line);
                                                }
                                                checked
                                            }
                                        };
                                        render_inline_spans(ui, text, &mut link_clicked, is_checked, ctx.vault, t);
                                    });
                                }
                                Block::Code { lang, code } => {
                                    Frame::NONE
                                        .fill(t.dock)
                                        .corner_radius(CornerRadius::same(t.radius_sm as u8))
                                        .inner_margin(Margin::same(10))
                                        .show(ui, |ui| {
                                            if !lang.is_empty() {
                                                theme::pill(ui, &lang.to_uppercase(), t.text_faint, t.field);
                                                ui.add_space(4.0);
                                            }
                                            ui.label(RichText::new(code.trim_end()).monospace().color(t.text).size(12.0));
                                        });
                                    ui.add_space(6.0);
                                }
                                Block::Table { headers, rows } => {
                                    egui::Grid::new(format!("preview_tbl_{}", item.line))
                                        .striped(true)
                                        .spacing(egui::vec2(12.0, 6.0))
                                        .show(ui, |ui| {
                                            for h in headers {
                                                ui.label(RichText::new(h).strong().color(t.accent_text));
                                            }
                                            ui.end_row();
                                            for r in rows {
                                                for cell in r {
                                                    ui.label(RichText::new(cell).color(t.text));
                                                }
                                                ui.end_row();
                                            }
                                        });
                                    ui.add_space(8.0);
                                }
                                Block::Quote(text) => {
                                    Frame::NONE
                                        .stroke(Stroke::new(2.0, t.accent))
                                        .inner_margin(Margin { left: 10, right: 0, top: 2, bottom: 2 })
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(text).italics().color(t.text_dim));
                                        });
                                    ui.add_space(6.0);
                                }
                                _ => {}
                            }
                        }

                        if let Some(line) = task_toggle {
                            if let Some(new_content) = toggle_task(&self.editor_buffer, line) {
                                self.editor_buffer = new_content;
                                self.editor_dirty = true;
                            }
                        }

                        if let Some(target) = link_clicked {
                            if let Ok(id) = NoteId::new(&target) {
                                self.open_note(id, ctx.vault, ctx.index);
                            }
                        }
                    });
            });
    }
}

fn render_inline_spans(
    ui: &mut Ui,
    text: &str,
    link_clicked: &mut Option<String>,
    strikethrough: bool,
    vault: &Vault,
    t: &Tokens,
) {
    let spans = parse_inline(text);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in spans {
            match span {
                Inline::Text(s) => {
                    let mut rt = RichText::new(s).color(t.text).size(13.5);
                    if strikethrough {
                        rt = rt.strikethrough();
                    }
                    ui.label(rt);
                }
                Inline::Bold(s) => {
                    let mut rt = RichText::new(s).strong().color(t.text).size(13.5);
                    if strikethrough {
                        rt = rt.strikethrough();
                    }
                    ui.label(rt);
                }
                Inline::Italic(s) => {
                    let mut rt = RichText::new(s).italics().color(t.text).size(13.5);
                    if strikethrough {
                        rt = rt.strikethrough();
                    }
                    ui.label(rt);
                }
                Inline::Code(s) => {
                    ui.label(RichText::new(format!(" {s} ")).monospace().color(t.accent_text).background_color(t.field).size(12.0));
                }
                Inline::Link(link) => {
                    let text = link.display_text();
                    let target = link.target.clone();
                    let target_id = NoteId::new(&target).ok();
                    let exists = target_id.as_ref().map_or(false, |id| vault.contains(id));
                    let color = if exists { t.accent_text } else { t.warning };
                    let btn = egui::Button::new(RichText::new(format!("[[{text}]]")).underline().color(color))
                        .frame(false);
                    if ui.add(btn).clicked() {
                        *link_clicked = Some(target);
                    }
                }
                _ => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_inline_format() {
        let mut plugin = MarkdownPlugin::new();
        plugin.active_note = Some(NoteId::new("Doc").unwrap());
        plugin.editor_buffer = "Hello world".to_string();

        plugin.editor_selection = Some((6, 11));
        plugin.apply_inline_format("**", "**", "bold");
        assert_eq!(plugin.editor_buffer, "Hello **world**");

        plugin.editor_cursor = plugin.editor_buffer.len();
        plugin.apply_inline_format(" [[", "]]", "Note");
        assert_eq!(plugin.editor_buffer, "Hello **world** [[Note]]");
    }

    #[test]
    fn test_insert_table_and_image() {
        let mut plugin = MarkdownPlugin::new();
        plugin.active_note = Some(NoteId::new("Doc").unwrap());
        plugin.editor_buffer = "# Doc\n".to_string();
        plugin.editor_cursor = plugin.editor_buffer.len();

        plugin.insert_table(2, 3);
        assert!(plugin.editor_buffer.contains("| Header 1 | Header 2 |"));
        assert!(plugin.editor_buffer.contains("| Cell 3-2 |"));

        plugin.insert_image("Architecture", "assets/arch.png");
        assert!(plugin.editor_buffer.contains("![Architecture](assets/arch.png)"));
    }
}
