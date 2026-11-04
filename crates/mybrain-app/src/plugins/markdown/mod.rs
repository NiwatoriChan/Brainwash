//! Markdown editor and live preview plugin for Brainwash.
//!
//! Provides 100% feature parity with Obsidian-style note editing:
//! - Full formatting toolbar (Headings dropdown, Bold, Italic, Strikethrough, Code, Lists, Tasks, Links, Image dialog, Table dropdown/modal, Event object, Query object, Callouts, Divider).
//! - Syntax highlighting layouter (`highlight_markdown`) in code editor.
//! - Wiki-link autocomplete popup dropdown when typing `[[`.
//! - Drag & drop note linking into editor buffer and preview.
//! - Scheduled Events widget for daily notes with "+ Add Event" and "Sync into Note".
//! - Obsidian frontmatter Properties card in preview.
//! - Obsidian Callouts (`> [!NOTE]`, `> [!TIP]`, `> [!WARNING]`, `> [!DANGER]`, etc.) & multi-line blockquotes with accent left bar `▎`.
//! - Markdown tables rendered as styled cards with inline formatting in cells.
//! - Embedded event object cards (`render_event_object_card`).
//! - Embedded query object cards (`render_query_object_card` for `events`, `todo`/`tasks`, `tag:`, `folder:`, `search:`).
//! - Note header with breadcrumbs, Unified Note Emoji & Color button, Daily Note badge & Calendar jump, Event badge & Agenda jump, More actions menu (`···`: Rename note, Delete note).
//! - Live task checkbox toggling.
//! - Multi-tab bar with icons, colors, and close buttons.
//! - Modals for Image insertion, Table creation, and Note renaming.

use chrono::NaiveDate;
use egui::text::LayoutJob;
use egui::{
    pos2, vec2, Color32, CornerRadius, FontId, Frame, Id, Margin, RichText, ScrollArea, Sense,
    Stroke, TextBuffer, TextFormat, Ui,
};
use mybrain_core::daily::DailyNotes;
use mybrain_core::events::{Event, EventStore};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::frontmatter::Frontmatter;
use mybrain_core::index::LinkIndex;
use mybrain_core::markdown::{
    parse_blocks, parse_inline, toggle_task, Block, Inline, ListMarker,
};
use mybrain_core::note::NoteId;
use mybrain_core::vault::Vault;
use mybrain_core::Note;

use crate::plugins::explorer::DraggedNote;
use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton,
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

    // Dialogs state
    pub show_insert_image_dialog: bool,
    pub insert_image_alt: String,
    pub insert_image_url: String,

    pub show_insert_table_dialog: bool,
    pub insert_table_cols: usize,
    pub insert_table_rows: usize,

    pub show_rename_dialog: bool,
    pub rename_target: Option<NoteId>,
    pub rename_new_name: String,

    pub clipboard_buffer: Option<String>,
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

            show_insert_image_dialog: false,
            insert_image_alt: String::new(),
            insert_image_url: String::new(),

            show_insert_table_dialog: false,
            insert_table_cols: 3,
            insert_table_rows: 3,

            show_rename_dialog: false,
            rename_target: None,
            rename_new_name: String::new(),

            clipboard_buffer: None,
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
        if !left.is_empty()
            && !left.ends_with('\n')
            && (snippet.starts_with("```") || snippet.starts_with('#') || snippet.starts_with("- "))
        {
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

    pub fn set_note_color(&mut self, id: &NoteId, color: Option<&str>, vault: &mut Vault, index: &mut LinkIndex) {
        let is_active = self.active_note.as_ref() == Some(id);
        let content = if is_active {
            self.editor_buffer.clone()
        } else if let Some(n) = vault.get(id) {
            n.content.clone()
        } else {
            return;
        };

        let (mut fm, body) = Frontmatter::parse(&content);
        if let Some(c) = color {
            fm.set("color", c);
        } else {
            fm.remove("color");
        }
        let new_content = if fm.is_empty() {
            body.trim_start_matches('\n').to_string()
        } else {
            format!("{}{}", fm.render(), body.trim_start_matches('\n'))
        };

        let _ = vault.save(id, &new_content);
        *index = LinkIndex::build(vault);
        if is_active {
            self.editor_buffer = new_content;
            self.editor_dirty = false;
        }
    }

    pub fn set_note_emoji(&mut self, id: &NoteId, emoji: Option<&str>, vault: &mut Vault, index: &mut LinkIndex) {
        let is_active = self.active_note.as_ref() == Some(id);
        let content = if is_active {
            self.editor_buffer.clone()
        } else if let Some(n) = vault.get(id) {
            n.content.clone()
        } else {
            return;
        };

        let (mut fm, body) = Frontmatter::parse(&content);
        if let Some(e) = emoji {
            fm.set("icon", e.trim());
        } else {
            fm.remove("icon");
        }
        let new_content = if fm.is_empty() {
            body.trim_start_matches('\n').to_string()
        } else {
            format!("{}{}", fm.render(), body.trim_start_matches('\n'))
        };

        let _ = vault.save(id, &new_content);
        *index = LinkIndex::build(vault);
        if is_active {
            self.editor_buffer = new_content;
            self.editor_dirty = false;
        }
    }

    pub fn sync_events_to_daily_note(&mut self, date: NaiveDate, vault: &mut Vault, index: &mut LinkIndex) {
        let event_store = EventStore::default();
        let events = event_store.on(vault, date);
        if events.is_empty() {
            return;
        }

        let daily = DailyNotes::default();
        let daily_id = match daily.open_or_create(vault, date) {
            Ok(id) => id,
            Err(_) => return,
        };

        if let Some(daily_note) = vault.get(&daily_id) {
            let mut content = daily_note.content.clone();
            let mut added = 0;
            for ev in events {
                if !content.contains(&ev.title) {
                    let bullet = ev.to_markdown_bullet();
                    if !content.ends_with('\n') && !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&bullet);
                    added += 1;
                }
            }
            if added > 0 {
                let _ = vault.save(&daily_id, &content);
                *index = LinkIndex::build(vault);
                if self.active_note.as_ref() == Some(&daily_id) {
                    self.editor_buffer = content;
                    self.editor_dirty = false;
                }
            }
        }
    }

    pub fn copy_selection(&mut self, ctx: &egui::Context) -> bool {
        if let Some((start, end)) = self.editor_selection {
            if start < end
                && end <= self.editor_buffer.len()
                && self.editor_buffer.is_char_boundary(start)
                && self.editor_buffer.is_char_boundary(end)
            {
                let text = self.editor_buffer[start..end].to_string();
                self.clipboard_buffer = Some(text.clone());
                ctx.copy_text(text.clone());
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(text);
                }
                return true;
            }
        }
        false
    }

    pub fn cut_selection(&mut self, ctx: &egui::Context) -> bool {
        if let Some((start, end)) = self.editor_selection {
            if start < end
                && end <= self.editor_buffer.len()
                && self.editor_buffer.is_char_boundary(start)
                && self.editor_buffer.is_char_boundary(end)
            {
                let text = self.editor_buffer[start..end].to_string();
                self.clipboard_buffer = Some(text.clone());
                ctx.copy_text(text.clone());
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(text);
                }
                self.editor_buffer.replace_range(start..end, "");
                self.editor_cursor = start;
                self.editor_selection = None;
                self.editor_dirty = true;
                return true;
            }
        }
        false
    }

    pub fn paste_from_clipboard(&mut self) -> bool {
        let text_to_paste = arboard::Clipboard::new()
            .ok()
            .and_then(|mut cb| cb.get_text().ok())
            .or_else(|| self.clipboard_buffer.clone());

        if let Some(paste_text) = text_to_paste {
            if !paste_text.is_empty() {
                if let Some((start, end)) = self.editor_selection {
                    if start < end
                        && end <= self.editor_buffer.len()
                        && self.editor_buffer.is_char_boundary(start)
                        && self.editor_buffer.is_char_boundary(end)
                    {
                        self.editor_buffer.replace_range(start..end, &paste_text);
                        self.editor_cursor = start + paste_text.len();
                        self.editor_selection = None;
                        self.editor_dirty = true;
                        return true;
                    }
                }
                let cursor = self.editor_cursor.min(self.editor_buffer.len());
                let cursor = if self.editor_buffer.is_char_boundary(cursor) {
                    cursor
                } else {
                    self.editor_buffer.len()
                };
                self.editor_buffer.insert_str(cursor, &paste_text);
                self.editor_cursor = cursor + paste_text.len();
                self.editor_selection = None;
                self.editor_dirty = true;
                return true;
            }
        }
        false
    }

    pub fn select_all(&mut self, ctx: &egui::Context, text_edit_id: egui::Id) {
        let len = self.editor_buffer.len();
        if len > 0 {
            self.editor_selection = Some((0, len));
            self.editor_cursor = len;
            if let Some(mut state) = egui::text_edit::TextEditState::load(ctx, text_edit_id) {
                let char_count = self.editor_buffer.chars().count();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(char_count),
                )));
                state.store(ctx, text_edit_id);
            }
        }
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

    fn view_type(&self) -> PluginViewType {
        PluginViewType::MainWorkspace
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        None
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
            PluginCommand {
                id: "rename_note".into(),
                title: "Rename Current Note".into(),
                shortcut: Some("F2".into()),
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
            "rename_note" => {
                if let Some(ref active) = self.active_note {
                    self.rename_target = Some(active.clone());
                    self.rename_new_name = active.as_str().to_string();
                    self.show_rename_dialog = true;
                }
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
            AppEvent::StatusMessage(msg) => {
                if let Some(stripped) = msg.strip_prefix("RENAME_NOTE:") {
                    if let Ok(id) = NoteId::new(stripped) {
                        self.rename_target = Some(id.clone());
                        self.rename_new_name = id.as_str().to_string();
                        self.show_rename_dialog = true;
                    }
                }
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
                    ui.label(
                        RichText::new("Choose a note from the File Explorer or press Ctrl+N to create one.")
                            .color(t.text_faint)
                            .size(13.0),
                    );
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
                    let note_color_name = note_fm.as_ref().and_then(|fm| fm.get("color")).map(|s| s.to_string());
                    let note_emoji = note_fm.as_ref().and_then(|fm| fm.get("icon")).map(|s| s.to_string());

                    let daily_notes = DailyNotes::default();
                    let daily_date = daily_notes.date_of(&active_id);
                    let is_daily = daily_date.is_some();
                    let event_info = note.and_then(Event::from_note);
                    let is_event = event_info.is_some();

                    let default_icon = if is_daily { "📅" } else if is_event { "🔔" } else { "📝" };
                    let current_icon = note_emoji.as_deref().unwrap_or(default_icon);
                    let icon_color = note_color.unwrap_or(if is_event { t.warning } else { t.accent });

                    // Unified Note Emoji & Color button
                    let mut color_to_set = None;
                    let mut emoji_to_set = None;
                    let mut open_more_emojis = false;
                    let mut reset_color_emoji = false;

                    ui.menu_button(
                        RichText::new(current_icon).size(15.0).color(icon_color),
                        |ui| {
                            ui.set_min_width(220.0);

                            // Section 1: Note Color
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Color:").size(11.5).color(t.text_faint));
                                ui.add_space(4.0);
                                if let Some(action) = theme::color_picker_dots(
                                    ui,
                                    &t,
                                    ctx.settings.effective_color_palette(),
                                    note_color_name.as_deref(),
                                ) {
                                    match action {
                                        theme::ColorPickerAction::SetColor(c) => {
                                            color_to_set = Some(c);
                                            ui.close();
                                        }
                                        theme::ColorPickerAction::OpenCustomColor => {
                                            ui.close();
                                        }
                                    }
                                }
                            });

                            ui.add_space(4.0);

                            // Section 2: Note Emoji
                            ui.label(RichText::new("Emoji:").size(11.5).color(t.text_faint));
                            ui.horizontal_wrapped(|ui| {
                                for em in ["📝", "📄", "💡", "🚀", "⭐", "📌", "🎯", "🔍", "🏷️", "💼", "📚", "🎨", "⚙️", "🌿", "⚡", "💬", "🔬", "📅"] {
                                    if ui.button(RichText::new(em).color(icon_color)).clicked() {
                                        emoji_to_set = Some(Some(em.to_string()));
                                        ui.close();
                                    }
                                }
                            });
                            ui.separator();
                            if ui.button("🔍 More Emojis…").clicked() {
                                open_more_emojis = true;
                                ui.close();
                            }
                            if (note_emoji.is_some() || note_color.is_some()) && ui.button("Reset Color & Emoji").clicked() {
                                reset_color_emoji = true;
                                ui.close();
                            }
                        },
                    )
                    .response
                    .on_hover_text("Change note icon & color");

                    if let Some(c) = color_to_set {
                        self.set_note_color(&active_id, c.as_deref(), ctx.vault, ctx.index);
                    }
                    if let Some(em) = emoji_to_set {
                        self.set_note_emoji(&active_id, em.as_deref(), ctx.vault, ctx.index);
                    }
                    if open_more_emojis {
                        ctx.emit(AppEvent::OpenModal(format!("emoji_note:{}", active_id.as_str())));
                    }
                    if reset_color_emoji {
                        self.set_note_color(&active_id, None, ctx.vault, ctx.index);
                        self.set_note_emoji(&active_id, None, ctx.vault, ctx.index);
                    }

                    // Title
                    ui.label(RichText::new(active_id.name()).color(t.text).strong().size(15.0));

                    // Daily Note badge & Calendar jump
                    if let Some(d_date) = daily_date {
                        ui.add_space(4.0);
                        theme::pill(ui, "Daily Note", t.accent_text, t.accent_soft);

                        let events_count = EventStore::default().on(ctx.vault, d_date).len();
                        if events_count > 0 {
                            theme::pill(ui, &format!("{events_count} event{}", if events_count == 1 { "" } else { "s" }), t.warning, t.accent_soft);
                            if theme::ghost_button(ui, &t, RichText::new("Sync Events").size(11.5).color(t.accent_text))
                                .on_hover_text("Synchronize scheduled events into note body")
                                .clicked()
                            {
                                self.sync_events_to_daily_note(d_date, ctx.vault, ctx.index);
                            }
                        }

                        if theme::ghost_button(ui, &t, RichText::new("Calendar ↗").size(11.5).color(t.accent_text))
                            .on_hover_text("Open Full Calendar View (Ctrl+2)")
                            .clicked()
                        {
                            ctx.activate_plugin("core.daily");
                        }
                    }

                    // Event badge & jump to Agenda
                    if let Some(ev) = event_info {
                        ui.add_space(4.0);
                        theme::pill(ui, &format!("Event: {}", ev.time_label()), t.warning, t.accent_soft);
                        if theme::ghost_button(ui, &t, RichText::new("Agenda ↗").size(11.5).color(t.warning))
                            .on_hover_text("View in Events Agenda")
                            .clicked()
                        {
                            ctx.activate_plugin("core.daily");
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // More actions menu (···)
                        ui.menu_button("···", |ui| {
                            ui.set_min_width(160.0);
                            if !is_daily && !is_event {
                                if ui.button("✏️ Rename / Move Note… (F2)").clicked() {
                                    self.rename_target = Some(active_id.clone());
                                    self.rename_new_name = active_id.as_str().to_string();
                                    self.show_rename_dialog = true;
                                    ui.close();
                                }
                                ui.separator();
                            }
                            if ui.button("🗑 Delete Note").clicked() {
                                let to_del = active_id.clone();
                                let _ = ctx.vault.delete(&to_del);
                                ctx.emit(AppEvent::NoteDeleted(to_del.clone()));
                                self.close_tab(&to_del, ctx.vault, ctx.index);
                                ctx.toast("Note deleted");
                                ui.close();
                            }
                        });

                        ui.separator();

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

        // 5. Render Modal Dialogs (Insert Image, Insert Table, Rename Note)
        self.render_dialogs(ui.ctx(), ctx, &t);
    }
}

impl MarkdownPlugin {
    fn render_tab_bar(&mut self, ui: &mut Ui, ctx: &mut PluginContext, t: &Tokens) {
        if self.open_tabs.is_empty() {
            return;
        }

        let mut tab_to_close = None;
        let mut tab_to_select = None;
        let daily_notes = DailyNotes::default();

        Frame::NONE
            .fill(t.dock)
            .inner_margin(Margin {
                left: 10,
                right: 10,
                top: 4,
                bottom: 0,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    for tab_id in &self.open_tabs {
                        let is_active = self.active_note.as_ref() == Some(tab_id);
                        let note = ctx.vault.get(tab_id);
                        let note_fm = note.as_ref().map(|n| n.frontmatter());
                        let note_color = note_fm.as_ref().and_then(|fm| fm.get("color")).and_then(crate::theme::parse_color_str);
                        let note_emoji = note_fm.as_ref().and_then(|fm| fm.get("icon")).map(|s| s.to_string());
                        let is_daily = daily_notes.date_of(tab_id).is_some();
                        let is_event = note.and_then(Event::from_note).is_some();

                        let bg_color = if is_active { t.canvas } else { Color32::TRANSPARENT };
                        let stroke = if is_active { Stroke::new(1.0, t.card_border) } else { Stroke::NONE };

                        let frame = Frame::NONE
                            .fill(bg_color)
                            .stroke(stroke)
                            .corner_radius(CornerRadius {
                                nw: t.radius_sm as u8,
                                ne: t.radius_sm as u8,
                                sw: 0,
                                se: 0,
                            })
                            .inner_margin(Margin {
                                left: 10,
                                right: 8,
                                top: 5,
                                bottom: 5,
                            });

                        let resp = frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;

                                let tab_icon = if let Some(ref emoji) = note_emoji {
                                    emoji.as_str()
                                } else if is_daily {
                                    "📅"
                                } else if is_event {
                                    "🔔"
                                } else {
                                    "📝"
                                };

                                let icon_color = note_color.unwrap_or(if is_active { t.accent } else { t.text_dim });
                                ui.label(RichText::new(tab_icon).color(icon_color).size(12.5));

                                let text_color = if is_active { t.text } else { t.text_dim };
                                let mut title_text = RichText::new(tab_id.name()).color(text_color).size(12.0);
                                if is_active {
                                    title_text = title_text.strong();
                                }
                                ui.label(title_text);

                                if is_active && self.editor_dirty {
                                    ui.label(RichText::new("●").color(t.accent).size(8.0));
                                }

                                let close_resp = ui.add(
                                    egui::Button::new(RichText::new("×").size(13.0).color(t.text_faint)).frame(false),
                                );
                                if close_resp.clicked() {
                                    tab_to_close = Some(tab_id.clone());
                                }
                            });
                        });

                        if resp.response.clicked() {
                            tab_to_select = Some(tab_id.clone());
                        }
                        if resp.response.middle_clicked() {
                            tab_to_close = Some(tab_id.clone());
                        }
                    }

                    if theme::ghost_button(ui, t, RichText::new("+").color(t.text_dim).size(13.0).strong())
                        .on_hover_text("New Note (Ctrl+N)")
                        .clicked()
                    {
                        ctx.emit(AppEvent::RequestNewNote(None));
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
            .inner_margin(Margin::symmetric(24, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    // 1. Typography & Headings
                    ui.menu_button(RichText::new("H ▾").color(t.text_dim).size(12.0).strong(), |ui| {
                        if ui.button("Heading 1  #").clicked() {
                            self.insert_snippet_into_active_note("# ");
                            ui.close();
                        }
                        if ui.button("Heading 2  ##").clicked() {
                            self.insert_snippet_into_active_note("## ");
                            ui.close();
                        }
                        if ui.button("Heading 3  ###").clicked() {
                            self.insert_snippet_into_active_note("### ");
                            ui.close();
                        }
                    });

                    if theme::ghost_button(ui, t, RichText::new("B").strong().color(t.text).size(12.0))
                        .on_hover_text("Bold (**text**)")
                        .clicked()
                    {
                        self.apply_inline_format("**", "**", "bold");
                    }

                    if theme::ghost_button(ui, t, RichText::new("I").italics().color(t.text).size(12.0))
                        .on_hover_text("Italic (*text*)")
                        .clicked()
                    {
                        self.apply_inline_format("*", "*", "italic");
                    }

                    if theme::ghost_button(ui, t, RichText::new("S").strikethrough().color(t.text_dim).size(12.0))
                        .on_hover_text("Strikethrough (~~text~~)")
                        .clicked()
                    {
                        self.apply_inline_format("~~", "~~", "text");
                    }

                    if theme::ghost_button(ui, t, RichText::new("`c`").monospace().color(t.accent_text).size(11.5))
                        .on_hover_text("Inline Code (`code`)")
                        .clicked()
                    {
                        self.apply_inline_format("`", "`", "code");
                    }

                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(2.0);

                    // 2. Lists & Tasks
                    if theme::ghost_button(ui, t, RichText::new("• List").color(t.text_dim).size(11.5))
                        .on_hover_text("Bullet list (- )")
                        .clicked()
                    {
                        self.insert_snippet_into_active_note("- ");
                    }

                    if theme::ghost_button(ui, t, RichText::new("1. List").color(t.text_dim).size(11.5))
                        .on_hover_text("Numbered list (1. )")
                        .clicked()
                    {
                        self.insert_snippet_into_active_note("1. ");
                    }

                    if theme::ghost_button(ui, t, RichText::new("Task").color(t.text).size(11.5))
                        .on_hover_text("Task Checkbox (- [ ] )")
                        .clicked()
                    {
                        self.insert_snippet_into_active_note("- [ ] ");
                    }

                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(2.0);

                    // 3. Links & Media
                    if theme::ghost_button(ui, t, RichText::new("Link").color(t.text_dim).size(11.5))
                        .on_hover_text("Insert [[Note Link]]")
                        .clicked()
                    {
                        self.apply_inline_format("[[", "]]", "Note");
                    }

                    if theme::ghost_button(ui, t, RichText::new("Image").color(t.accent_text).size(11.5).strong())
                        .on_hover_text("Insert Image (![alt](url))")
                        .clicked()
                    {
                        self.show_insert_image_dialog = true;
                    }

                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(2.0);

                    // 4. Rich Objects
                    ui.menu_button(
                        RichText::new("Table ▾").color(t.accent_text).size(11.5).strong(),
                        |ui| {
                            ui.set_min_width(160.0);
                            if ui.button("2 × 2 Table").clicked() {
                                self.insert_table(2, 2);
                                ui.close();
                            }
                            if ui.button("3 × 3 Table").clicked() {
                                self.insert_table(3, 3);
                                ui.close();
                            }
                            if ui.button("4 × 4 Table").clicked() {
                                self.insert_table(4, 4);
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Custom Table...").clicked() {
                                self.show_insert_table_dialog = true;
                                ui.close();
                            }
                        },
                    );

                    if theme::ghost_button(ui, t, RichText::new("Event").color(t.warning).size(11.5))
                        .on_hover_text("Insert embedded Event Object block")
                        .clicked()
                    {
                        self.insert_snippet_into_active_note("```event\ntitle: Team Sync\ndate: 2026-11-01\nstart: 10:00\nend: 11:00\nlocation: Conference Room A\n```\n");
                    }

                    ui.menu_button(
                        RichText::new("Query ▾").color(t.accent_text).size(11.5),
                        |ui| {
                            ui.set_min_width(180.0);
                            if ui.button("Upcoming Events").clicked() {
                                self.insert_snippet_into_active_note("```query\nevents\n```\n");
                                ui.close();
                            }
                            if ui.button("Open Tasks").clicked() {
                                self.insert_snippet_into_active_note("```query\ntodo\n```\n");
                                ui.close();
                            }
                            if ui.button("Filter by Tag").clicked() {
                                self.insert_snippet_into_active_note("```query\ntag: #project\n```\n");
                                ui.close();
                            }
                            if ui.button("Filter by Folder").clicked() {
                                self.insert_snippet_into_active_note("```query\nfolder: Projects\n```\n");
                                ui.close();
                            }
                            if ui.button("Search Query").clicked() {
                                self.insert_snippet_into_active_note("```query\nsearch: keyword\n```\n");
                                ui.close();
                            }
                        },
                    );

                    ui.menu_button(
                        RichText::new("Callout ▾").color(t.text_dim).size(11.5),
                        |ui| {
                            ui.set_min_width(140.0);
                            if ui.button("Note [!NOTE]").clicked() {
                                self.insert_snippet_into_active_note("> [!NOTE]\n> Key reminder or note.\n");
                                ui.close();
                            }
                            if ui.button("Tip [!TIP]").clicked() {
                                self.insert_snippet_into_active_note("> [!TIP]\n> Helpful tip or hint.\n");
                                ui.close();
                            }
                            if ui.button("Warning [!WARNING]").clicked() {
                                self.insert_snippet_into_active_note("> [!WARNING]\n> Important caution or warning.\n");
                                ui.close();
                            }
                            if ui.button("Danger [!DANGER]").clicked() {
                                self.insert_snippet_into_active_note("> [!DANGER]\n> Critical alert or error.\n");
                                ui.close();
                            }
                        },
                    );

                    if theme::ghost_button(ui, t, RichText::new("─").color(t.text_faint).size(12.0))
                        .on_hover_text("Divider Rule (---)")
                        .clicked()
                    {
                        self.insert_snippet_into_active_note("\n---\n");
                    }
                });
            });
    }

    fn render_editor_pane(&mut self, ui: &mut Ui, ctx: &mut PluginContext, t: &Tokens) {
        let is_dragged = egui::DragAndDrop::has_payload_of_type::<DraggedNote>(ui.ctx());

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
            let avail_width = ui.available_width();
            ui.set_max_width(avail_width);

            if is_dragged {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Drop note anywhere to insert [[link]]").strong().color(t.accent_text).size(12.0));
                });
                ui.add_space(6.0);
            }

            ScrollArea::vertical()
                .id_salt("editor_scroll")
                .max_width(avail_width)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width());

                    // Scheduled Events card if this is a daily note
                    let daily_notes = DailyNotes::default();
                    if let Some(ref active_id) = self.active_note {
                        if let Some(d_date) = daily_notes.date_of(active_id) {
                            render_daily_events_widget(ui, self, ctx, d_date, t);
                            ui.add_space(12.0);
                        }
                    }

                    let editor_font_size = ctx.settings.editor_font_size.clamp(10.0, 24.0);
                    let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
                        let mut layout_job = highlight_markdown(buf.as_str(), t, editor_font_size);
                        layout_job.wrap.max_width = wrap_width;
                        ui.ctx().fonts_mut(|f| f.layout_job(layout_job))
                    };

                    let output = egui::TextEdit::multiline(&mut self.editor_buffer)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter)
                        .frame(Frame::NONE)
                        .show(ui);

                    if let Some(cursor_range) = output.cursor_range {
                        let char_to_byte = |char_idx: usize| -> usize {
                            self.editor_buffer
                                .char_indices()
                                .nth(char_idx)
                                .map(|(b, _)| b)
                                .unwrap_or(self.editor_buffer.len())
                        };
                        let primary_byte = char_to_byte(cursor_range.primary.index.0);
                        let secondary_byte = char_to_byte(cursor_range.secondary.index.0);
                        self.editor_cursor = primary_byte;
                        let (start, end) = if primary_byte <= secondary_byte {
                            (primary_byte, secondary_byte)
                        } else {
                            (secondary_byte, primary_byte)
                        };
                        self.editor_selection = if start < end {
                            Some((start, end))
                        } else {
                            None
                        };
                    }

                    if output.response.changed() {
                        self.editor_dirty = true;
                    }

                    // Context Menu (Right Click) with Couper, Copier, Coller, Tout selectionner
                    let text_edit_id = output.response.id;
                    output.response.context_menu(|ui| {
                        ui.set_min_width(185.0);

                        let has_selection = self.editor_selection.is_some();
                        let clipboard_has_text = arboard::Clipboard::new()
                            .ok()
                            .and_then(|mut cb| cb.get_text().ok())
                            .map(|s| !s.is_empty())
                            .unwrap_or(self.clipboard_buffer.is_some());

                        // 1. Couper
                        if theme::context_menu_item(ui, t, theme::paint_cut_icon, "Couper", "Ctrl+X", has_selection).clicked() {
                            self.cut_selection(ui.ctx());
                            ui.close();
                        }

                        // 2. Copier
                        if theme::context_menu_item(ui, t, theme::paint_copy_icon, "Copier", "Ctrl+C", has_selection).clicked() {
                            self.copy_selection(ui.ctx());
                            ui.close();
                        }

                        // 3. Coller
                        if theme::context_menu_item(ui, t, theme::paint_paste_icon, "Coller", "Ctrl+V", clipboard_has_text).clicked() {
                            self.paste_from_clipboard();
                            ui.close();
                        }

                        ui.separator();

                        // 4. Tout selectionner
                        let has_content = !self.editor_buffer.is_empty();
                        if theme::context_menu_item(ui, t, theme::paint_select_all_icon, "Tout selectionner", "Ctrl+A", has_content).clicked() {
                            self.select_all(ui.ctx(), text_edit_id);
                            ui.close();
                        }
                    });

                    // Keyboard shortcuts handling
                    if output.response.has_focus() {
                        let is_cmd = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                        if is_cmd && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C)) {
                            self.copy_selection(ui.ctx());
                        } else if is_cmd && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::X)) {
                            self.cut_selection(ui.ctx());
                        } else if is_cmd && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V)) {
                            self.paste_from_clipboard();
                        } else if is_cmd && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)) {
                            self.select_all(ui.ctx(), text_edit_id);
                        }
                    }

                    // Wiki-link autocomplete popup when typing `[[`
                    render_link_autocomplete(ui, self, ctx, &output, t);

                    ui.add_space(80.0);
                });
        });

        // Drop target handling
        if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
            if frame_resp.response.rect.contains(hover_pos) && ui.input(|i| i.pointer.any_released()) {
                if let Some(dragged) = egui::DragAndDrop::take_payload::<DraggedNote>(ui.ctx()) {
                    let link_str = format!("[[{}]]", dragged.name);
                    if !self.editor_buffer.is_empty()
                        && !self.editor_buffer.ends_with(' ')
                        && !self.editor_buffer.ends_with('\n')
                    {
                        self.editor_buffer.push(' ');
                    }
                    self.editor_buffer.push_str(&link_str);
                    self.editor_buffer.push(' ');
                    self.editor_dirty = true;
                    ctx.toast(format!("Linked note: {link_str}"));
                }
            }
        }
    }

    fn render_preview_pane(&mut self, ui: &mut Ui, ctx: &mut PluginContext, t: &Tokens) {
        let is_dragged = egui::DragAndDrop::has_payload_of_type::<DraggedNote>(ui.ctx());

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
            let avail_width = ui.available_width();
            ui.set_max_width(avail_width);

            if is_dragged {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Drop note anywhere to insert [[link]]").strong().color(t.accent_text).size(12.0));
                });
                ui.add_space(6.0);
            }

            ScrollArea::vertical()
                .id_salt("preview_scroll")
                .max_width(avail_width)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let content_width = ui.available_width();
                    ui.vertical(|ui| {
                        ui.set_max_width(content_width);

                        // 1. Obsidian Frontmatter Properties Card
                        let (frontmatter, _) = Frontmatter::parse(&self.editor_buffer);
                        if !frontmatter.is_empty() {
                            render_properties_card(ui, &frontmatter, t);
                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(10.0);
                        }

                        // Scheduled Events card if daily note
                        let daily_notes = DailyNotes::default();
                        if let Some(ref active_id) = self.active_note {
                            if let Some(d_date) = daily_notes.date_of(active_id) {
                                render_daily_events_widget(ui, self, ctx, d_date, t);
                                ui.add_space(12.0);
                            }
                        }

                        // 2. Note title if not starting with H1
                        let blocks = parse_blocks(&self.editor_buffer);
                        let first_is_h1 = blocks.first().map_or(false, |b| matches!(b.block, Block::Heading { level: 1, .. }));
                        if !first_is_h1 {
                            if let Some(ref active_id) = self.active_note {
                                ui.label(RichText::new(active_id.name()).size(26.0).strong().color(t.text));
                                ui.add_space(8.0);
                            }
                        }

                        let mut task_to_toggle: Option<usize> = None;
                        let mut note_to_open: Option<String> = None;
                        let mut date_to_open: Option<NaiveDate> = None;
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
                                    render_inline_spans(ui, text, &mut note_to_open, false, ctx.vault, t);
                                    ui.add_space(6.0);
                                }
                                Block::ListItem { indent, marker, text } => {
                                    ui.horizontal_top(|ui| {
                                        ui.set_max_width(ui.available_width());
                                        if indent > 0 {
                                            ui.add_space(indent as f32 * 20.0);
                                        }
                                        let is_task_checked = match &marker {
                                            ListMarker::Bullet => {
                                                let (marker_rect, _) = ui.allocate_exact_size(vec2(16.0, 18.0), Sense::hover());
                                                let center = pos2(marker_rect.center().x, marker_rect.top() + 9.5);
                                                ui.painter().circle_filled(center, 2.8, t.accent);
                                                false
                                            }
                                            ListMarker::Ordered(n) => {
                                                ui.allocate_ui_with_layout(vec2(22.0, 18.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.label(RichText::new(format!("{n}.")).color(t.text_dim).size(12.0).strong());
                                                });
                                                ui.add_space(4.0);
                                                false
                                            }
                                            ListMarker::Task { checked } => {
                                                let mut is_checked = *checked;
                                                if ui.checkbox(&mut is_checked, "").changed() {
                                                    task_to_toggle = Some(line_no);
                                                }
                                                ui.add_space(2.0);
                                                *checked
                                            }
                                        };
                                        let text_width = ui.available_width();
                                        ui.allocate_ui_with_layout(vec2(text_width, 18.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                            ui.set_max_width(text_width);
                                            render_inline_spans(ui, text, &mut note_to_open, is_task_checked, ctx.vault, t);
                                        });
                                    });
                                    ui.add_space(3.0);
                                }
                                Block::Quote(text) => {
                                    render_quote_or_callout(ui, &text, &mut note_to_open, ctx.vault, t);
                                    ui.add_space(8.0);
                                }
                                Block::Table { headers, rows } => {
                                    render_markdown_table(
                                        ui,
                                        &headers,
                                        &rows,
                                        line_no,
                                        &mut note_to_open,
                                        ctx.vault,
                                        t,
                                    );
                                    ui.add_space(10.0);
                                }
                                Block::Code { lang, code } => {
                                    if lang.eq_ignore_ascii_case("event") {
                                        render_event_object_card(
                                            ui,
                                            self.active_note.as_ref(),
                                            &code,
                                            line_no,
                                            t,
                                            &mut date_to_open,
                                            &mut open_agenda,
                                        );
                                    } else if lang.eq_ignore_ascii_case("query") {
                                        render_query_object_card(
                                            ui,
                                            ctx.vault,
                                            &code,
                                            t,
                                            &mut note_to_open,
                                        );
                                    } else {
                                        Frame::NONE
                                            .fill(t.dock)
                                            .corner_radius(CornerRadius::same(t.radius_sm as u8))
                                            .inner_margin(Margin::same(12))
                                            .show(ui, |ui| {
                                                ui.set_max_width(ui.available_width());
                                                if !lang.is_empty() {
                                                    ui.horizontal(|ui| {
                                                        theme::pill(ui, &lang.to_uppercase(), t.text_faint, t.field);
                                                    });
                                                    ui.add_space(4.0);
                                                }
                                                ScrollArea::horizontal()
                                                    .id_salt(format!("code_block_{line_no}"))
                                                    .max_width(ui.available_width())
                                                    .show(ui, |ui| {
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

                        // Task toggle action
                        if let Some(line) = task_to_toggle {
                            if let Some(new_content) = toggle_task(&self.editor_buffer, line) {
                                self.editor_buffer = new_content;
                                self.editor_dirty = true;
                            }
                        }

                        // Open note link
                        if let Some(target) = note_to_open {
                            if let Ok(id) = NoteId::new(&target) {
                                if ctx.vault.get(&id).is_some() {
                                    self.open_note(id, ctx.vault, ctx.index);
                                } else {
                                    let _ = ctx.vault.create(id.clone(), "");
                                    *ctx.index = LinkIndex::build(ctx.vault);
                                    self.open_note(id, ctx.vault, ctx.index);
                                }
                            }
                        }

                        // Daily jump
                        if let Some(date) = date_to_open {
                            let daily = DailyNotes::default();
                            if let Ok(id) = daily.open_or_create(ctx.vault, date) {
                                *ctx.index = LinkIndex::build(ctx.vault);
                                self.open_note(id, ctx.vault, ctx.index);
                            }
                        }

                        // Agenda jump
                        if open_agenda {
                            ctx.activate_plugin("core.daily");
                        }

                        ui.add_space(80.0);
                    });
                });
        });

        // Drop target handling
        if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
            if frame_resp.response.rect.contains(hover_pos) && ui.input(|i| i.pointer.any_released()) {
                if let Some(dragged) = egui::DragAndDrop::take_payload::<DraggedNote>(ui.ctx()) {
                    let link_str = format!("[[{}]]", dragged.name);
                    if !self.editor_buffer.is_empty()
                        && !self.editor_buffer.ends_with(' ')
                        && !self.editor_buffer.ends_with('\n')
                    {
                        self.editor_buffer.push(' ');
                    }
                    self.editor_buffer.push_str(&link_str);
                    self.editor_buffer.push(' ');
                    self.editor_dirty = true;
                    ctx.toast(format!("Linked note: {link_str}"));
                }
            }
        }
    }

    fn render_dialogs(&mut self, egui_ctx: &egui::Context, ctx: &mut PluginContext, t: &Tokens) {
        // 1. Insert Image Dialog
        if self.show_insert_image_dialog {
            let mut should_close = false;
            let mut should_insert = false;

            let modal = egui::Modal::new(Id::unique("md_insert_image_modal"))
                .backdrop_color(Color32::from_black_alpha(150))
                .frame(theme::modal_frame(t));

            let modal_resp = modal.show(egui_ctx, |ui| {
                ui.set_width(420.0);

                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Insert Image").color(t.text).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon_close_button(ui, t).clicked() {
                            should_close = true;
                        }
                    });
                });

                ui.add_space(10.0);

                ui.label(RichText::new("Alt Text / Caption").color(t.text_dim).size(12.0));
                field_frame(t).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.insert_image_alt)
                            .hint_text("Diagram or Screenshot")
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                });

                ui.add_space(8.0);

                ui.label(RichText::new("Image Path or URL").color(t.text_dim).size(12.0));
                field_frame(t).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.insert_image_url)
                            .hint_text("images/architecture.png or https://example.com/photo.png")
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                });

                ui.add_space(6.0);
                ui.label(
                    RichText::new("Tip: Inserts standard Markdown ![alt](url), rendered as an image card in live preview.")
                        .color(t.text_faint)
                        .size(11.0)
                        .italics(),
                );

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let can_insert = !self.insert_image_url.trim().is_empty();
                    if ui.add_enabled(
                        can_insert,
                        egui::Button::new(RichText::new("Insert Image").color(Color32::WHITE).strong())
                            .fill(t.accent)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    ).clicked() {
                        should_insert = true;
                        should_close = true;
                    }

                    if theme::ghost_button(ui, t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                        should_close = true;
                    }
                });
            });

            if should_insert {
                let alt = self.insert_image_alt.trim().to_string();
                let url = self.insert_image_url.trim().to_string();
                self.insert_image(&alt, &url);
                self.insert_image_alt.clear();
                self.insert_image_url.clear();
            }

            if modal_resp.should_close() || should_close {
                self.show_insert_image_dialog = false;
            }
        }

        // 2. Insert Custom Table Dialog
        if self.show_insert_table_dialog {
            let mut should_close = false;
            let mut should_insert = false;

            let modal = egui::Modal::new(Id::unique("md_insert_table_modal"))
                .backdrop_color(Color32::from_black_alpha(150))
                .frame(theme::modal_frame(t));

            let modal_resp = modal.show(egui_ctx, |ui| {
                ui.set_width(380.0);

                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Insert Markdown Table").color(t.text).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon_close_button(ui, t).clicked() {
                            should_close = true;
                        }
                    });
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Columns:").color(t.text_dim).size(13.0));
                    ui.add(egui::DragValue::new(&mut self.insert_table_cols).range(1..=10).speed(0.1));
                    ui.add_space(20.0);
                    ui.label(RichText::new("Rows:").color(t.text_dim).size(13.0));
                    ui.add(egui::DragValue::new(&mut self.insert_table_rows).range(1..=30).speed(0.1));
                });

                ui.add_space(10.0);

                Frame::NONE
                    .fill(t.field)
                    .stroke(Stroke::new(1.0, t.field_border))
                    .corner_radius(CornerRadius::same(t.radius_sm as u8))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Generates a {} × {} table ({} headers + {} data rows)",
                                self.insert_table_cols,
                                self.insert_table_rows,
                                self.insert_table_cols,
                                self.insert_table_rows
                            ))
                            .color(t.text_dim)
                            .size(12.0),
                        );
                    });

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    if ui.add(
                        egui::Button::new(RichText::new("Insert Table").color(Color32::WHITE).strong())
                            .fill(t.accent)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    ).clicked() {
                        should_insert = true;
                        should_close = true;
                    }

                    if theme::ghost_button(ui, t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                        should_close = true;
                    }
                });
            });

            if should_insert {
                self.insert_table(self.insert_table_cols, self.insert_table_rows);
            }

            if modal_resp.should_close() || should_close {
                self.show_insert_table_dialog = false;
            }
        }

        // 3. Rename Note Dialog
        if self.show_rename_dialog {
            let mut should_close = false;
            let mut rename_action: Option<(NoteId, String)> = None;

            let modal = egui::Modal::new(Id::unique("md_rename_note_modal"))
                .backdrop_color(Color32::from_black_alpha(150))
                .frame(theme::modal_frame(t));

            let modal_resp = modal.show(egui_ctx, |ui| {
                ui.set_width(380.0);

                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Rename or Move Note").color(t.text).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon_close_button(ui, t).clicked() {
                            should_close = true;
                        }
                    });
                });

                ui.add_space(6.0);
                if let Some(target) = &self.rename_target {
                    ui.label(
                        RichText::new(format!("Current: {}", target.as_str()))
                            .color(t.accent_text)
                            .size(11.5),
                    );
                }

                ui.add_space(6.0);
                ui.label(
                    RichText::new("New title or folder path (all [[wiki links]] will update automatically):")
                        .color(t.text_dim)
                        .size(12.0),
                );

                ui.add_space(4.0);
                let mut submitted = false;
                field_frame(t).show(ui, |ui| {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.rename_new_name)
                            .hint_text("Projects/Roadmap")
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                    edit.request_focus();
                    submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                });

                let typed_name = self.rename_new_name.trim();
                let can_rename = !typed_name.is_empty()
                    && self.rename_target.as_ref().map(|t| t.as_str()) != Some(typed_name);

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            can_rename,
                            egui::Button::new(RichText::new("Rename Note").color(Color32::WHITE).strong())
                                .fill(t.accent)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                        )
                        .clicked()
                        || (submitted && can_rename)
                    {
                        if let Some(from) = self.rename_target.clone() {
                            rename_action = Some((from, self.rename_new_name.clone()));
                        }
                        should_close = true;
                    }

                    if theme::ghost_button(ui, t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                        should_close = true;
                    }
                });
            });

            if let Some((from, new_name)) = rename_action {
                if let Ok(new_id) = NoteId::new(&new_name) {
                    if let Ok(_updated_links) = ctx.vault.rename(&from, new_id.clone()) {
                        *ctx.index = LinkIndex::build(ctx.vault);
                        ctx.emit(AppEvent::NoteRenamed { old: from.clone(), new: new_id.clone() });
                        if let Some(pos) = self.open_tabs.iter().position(|tab| tab == &from) {
                            self.open_tabs[pos] = new_id.clone();
                        }
                        if self.active_note.as_ref() == Some(&from) {
                            self.active_note = Some(new_id.clone());
                            if let Some(n) = ctx.vault.get(&new_id) {
                                self.editor_buffer = n.content.clone();
                            }
                        }
                        ctx.toast("Note renamed");
                    }
                }
            }

            if modal_resp.should_close() || should_close {
                self.show_rename_dialog = false;
                self.rename_target = None;
            }
        }
    }
}

fn field_frame(t: &Tokens) -> Frame {
    Frame::NONE
        .fill(t.field)
        .stroke(Stroke::new(1.0, t.field_border))
        .corner_radius(CornerRadius::same(t.radius_sm as u8))
        .inner_margin(Margin::symmetric(10, 6))
}

fn highlight_inline_links_and_tags(
    line: &str,
    job: &mut LayoutJob,
    default_font: &FontId,
    t: &Tokens,
) {
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

fn highlight_markdown(text: &str, t: &Tokens, font_size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let default_font = FontId::monospace(font_size);
    let heading_font = FontId::monospace(font_size + 1.5);

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let leading_len = line.len() - trimmed.len();
        let leading = &line[..leading_len];

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
        } else if trimmed.starts_with("- ")
            || trimmed.starts_with("-\t")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("*\t")
            || trimmed.starts_with("+ ")
            || trimmed.starts_with("+\t")
            || trimmed == "-\n"
            || trimmed == "-"
            || trimmed == "*\n"
            || trimmed == "*"
            || trimmed == "+\n"
            || trimmed == "+"
        {
            if leading_len > 0 {
                job.append(
                    leading,
                    0.0,
                    TextFormat {
                        font_id: default_font.clone(),
                        color: t.text,
                        ..Default::default()
                    },
                );
            }
            let marker_len = if trimmed.len() >= 2 && (trimmed.as_bytes()[1] == b' ' || trimmed.as_bytes()[1] == b'\t') {
                2
            } else {
                1
            };
            let marker = &trimmed[..marker_len];
            let after_marker = &trimmed[marker_len..];

            job.append(
                marker,
                0.0,
                TextFormat {
                    font_id: default_font.clone(),
                    color: t.accent,
                    ..Default::default()
                },
            );

            let rest_line = if let Some(rest_task) = after_marker.strip_prefix("[ ] ") {
                job.append(
                    "[ ] ",
                    0.0,
                    TextFormat {
                        font_id: default_font.clone(),
                        color: t.text_dim,
                        ..Default::default()
                    },
                );
                rest_task
            } else if let Some(rest_task) = after_marker.strip_prefix("[x] ").or_else(|| after_marker.strip_prefix("[X] ")) {
                job.append(
                    &after_marker[..4],
                    0.0,
                    TextFormat {
                        font_id: default_font.clone(),
                        color: t.success,
                        ..Default::default()
                    },
                );
                rest_task
            } else {
                after_marker
            };

            highlight_inline_links_and_tags(rest_line, &mut job, &default_font, t);
        } else {
            highlight_inline_links_and_tags(line, &mut job, &default_font, t);
        }
    }
    job
}

fn render_link_autocomplete(
    ui: &mut Ui,
    plugin: &mut MarkdownPlugin,
    ctx: &PluginContext,
    output: &egui::text_edit::TextEditOutput,
    t: &Tokens,
) {
    if let Some(cursor_range) = output.cursor_range {
        let cursor_char_idx = cursor_range.primary.index.0;
        let text = plugin.editor_buffer.clone();

        let byte_offset = text
            .char_indices()
            .nth(cursor_char_idx)
            .map(|(b, _)| b)
            .unwrap_or(text.len());

        if let Some((start_pos, query)) = mybrain_core::links::open_link_at(&text, byte_offset) {
            let query_lower = query.to_lowercase();
            let matches: Vec<String> = ctx
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
                    .show(ui, |ui| {
                        ui.label(RichText::new("Link to note:").color(t.text_faint).size(11.0));
                        for name in matches {
                            if ui.button(RichText::new(&name).color(t.accent_text).size(12.0)).clicked() {
                                chosen_note = Some(name);
                            }
                        }
                    });

                if let Some(chosen) = chosen_note {
                    let end_pos = byte_offset;
                    let replacement = format!("[[{chosen}]]");
                    let mut new_buf = String::with_capacity(text.len() + replacement.len());
                    new_buf.push_str(&text[..start_pos]);
                    new_buf.push_str(&replacement);
                    new_buf.push_str(&text[end_pos..]);
                    plugin.editor_buffer = new_buf;
                    plugin.editor_dirty = true;
                    plugin.editor_cursor = start_pos + replacement.len();
                }
            }
        }
    }
}

fn render_daily_events_widget(
    ui: &mut Ui,
    plugin: &mut MarkdownPlugin,
    ctx: &mut PluginContext,
    date: NaiveDate,
    t: &Tokens,
) {
    let event_store = EventStore::default();
    let events = event_store.on(ctx.vault, date);
    let card = theme::card_frame(t, false).inner_margin(Margin::symmetric(14, 10));

    let mut event_to_open = None;
    let mut sync_clicked = false;

    card.show(ui, |ui| {
        ui.horizontal(|ui| {
            theme::section_label(ui, t, &format!("Scheduled Events ({})", events.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
        plugin.open_note(id, ctx.vault, ctx.index);
    }
    if sync_clicked {
        plugin.sync_events_to_daily_note(date, ctx.vault, ctx.index);
    }
}

fn render_properties_card(ui: &mut Ui, frontmatter: &Frontmatter, t: &Tokens) {
    let card = theme::card_frame(t, false).inner_margin(Margin::symmetric(14, 10));

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

fn render_markdown_table(
    ui: &mut Ui,
    headers: &[&str],
    rows: &[Vec<&str>],
    line_no: usize,
    note_to_open: &mut Option<String>,
    vault: &Vault,
    t: &Tokens,
) {
    let card = theme::card_frame(t, false).inner_margin(Margin::symmetric(12, 8));
    card.show(ui, |ui| {
        ui.set_max_width(ui.available_width());
        egui::Grid::new(format!("md_table_{line_no}"))
            .striped(true)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                for h in headers {
                    ui.label(RichText::new(*h).strong().color(t.accent_text).size(12.5));
                }
                ui.end_row();

                for row in rows {
                    for col_idx in 0..headers.len() {
                        let cell = row.get(col_idx).copied().unwrap_or("");
                        render_inline_spans(ui, cell, note_to_open, false, vault, t);
                    }
                    ui.end_row();
                }
            });
    });
}

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

            let mut lines_iter = after.lines();
            let first_line_title = lines_iter.next().unwrap_or("").trim();
            let remaining_body: Vec<&str> = lines_iter.collect();

            Frame::NONE
                .fill(t.accent_soft)
                .stroke(Stroke::new(1.0, t.accent_border))
                .corner_radius(CornerRadius::same(t.radius_sm as u8))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width());
                    ui.horizontal(|ui| {
                        theme::pill(ui, label, accent_color, t.card);
                        if !first_line_title.is_empty() && !remaining_body.is_empty() {
                            ui.label(RichText::new(first_line_title).strong().color(t.text).size(12.5));
                        }
                    });
                    ui.add_space(4.0);
                    if remaining_body.is_empty() {
                        if !first_line_title.is_empty() {
                            render_inline_spans(ui, first_line_title, note_to_open, false, vault, t);
                        }
                    } else {
                        for line in remaining_body {
                            if !line.trim().is_empty() {
                                render_inline_spans(ui, line, note_to_open, false, vault, t);
                            }
                        }
                    }
                });
            return;
        }
    }

    // Standard blockquote with accent left bar
    Frame::NONE
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            for line in text.lines() {
                ui.horizontal_top(|ui| {
                    ui.set_max_width(ui.available_width());
                    ui.label(RichText::new("▎").strong().color(t.accent).size(13.0));
                    render_inline_spans(ui, line, note_to_open, false, vault, t);
                });
            }
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
    let avail_width = ui.available_width();
    ui.horizontal_wrapped(|ui| {
        ui.set_max_width(avail_width);
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
                Inline::Strikethrough(txt) => {
                    ui.label(RichText::new(txt).color(t.text_dim).strikethrough());
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

fn render_event_object_card(
    ui: &mut Ui,
    active_note: Option<&NoteId>,
    code: &str,
    line_no: usize,
    t: &Tokens,
    open_daily: &mut Option<NaiveDate>,
    open_agenda: &mut bool,
) {
    let dummy_id = active_note.cloned().unwrap_or_else(|| NoteId::new("Note").unwrap());
    let dummy_note = Note::new(dummy_id, "");
    let lines: Vec<&str> = code.lines().collect();
    let ev = Event::parse_block(&dummy_note, line_no, &lines);

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

fn render_query_object_card(
    ui: &mut Ui,
    vault: &Vault,
    code: &str,
    t: &Tokens,
    note_to_open: &mut Option<String>,
) {
    let query_trim = code.trim();
    let query_lower = query_trim.to_lowercase();
    let event_store = EventStore::default();

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

    #[test]
    fn test_markdown_syntax_highlighting_job() {
        let t = Tokens::for_kind(crate::theme::ThemeKind::default());
        let markdown = "# Title\n\n```rust\nlet x = 42;\n```\n\n> [!NOTE]\n> A callout\n\n- item 1\n  - [ ] task\n- [x] done\n\nSee [[OtherNote]] here\n";
        let job = highlight_markdown(markdown, &t, 13.0);
        assert!(!job.text.is_empty());
        assert_eq!(job.text, markdown);
    }

    #[test]
    fn test_copy_cut_paste_and_select_all() {
        let ctx = egui::Context::default();
        let mut plugin = MarkdownPlugin::new();
        plugin.active_note = Some(NoteId::new("Test").unwrap());
        plugin.editor_buffer = "The quick brown fox".to_string();

        // 1. Copy selection
        plugin.editor_selection = Some((4, 9)); // "quick"
        assert!(plugin.copy_selection(&ctx));

        // 2. Cut selection
        assert!(plugin.cut_selection(&ctx));
        assert_eq!(plugin.editor_buffer, "The  brown fox");
        assert_eq!(plugin.editor_selection, None);
        assert_eq!(plugin.editor_cursor, 4);

        // 3. Paste
        assert!(plugin.paste_from_clipboard());
        assert_eq!(plugin.editor_buffer, "The quick brown fox");
        assert_eq!(plugin.editor_cursor, 9);

        // 4. Select all
        let dummy_id = Id::NULL.with("dummy_editor");
        plugin.select_all(&ctx, dummy_id);
        assert_eq!(plugin.editor_selection, Some((0, 19)));
        assert_eq!(plugin.editor_cursor, 19);
    }

    #[test]
    fn test_context_menu_rendering_with_icons() {
        let t = Tokens::for_kind(crate::theme::ThemeKind::default());
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Test all 4 context menu items render with vector icons
                let _r1 = theme::context_menu_item(ui, &t, theme::paint_cut_icon, "Couper", "Ctrl+X", true);
                let _r2 = theme::context_menu_item(ui, &t, theme::paint_copy_icon, "Copier", "Ctrl+C", true);
                let _r3 = theme::context_menu_item(ui, &t, theme::paint_paste_icon, "Coller", "Ctrl+V", true);
                let _r4 = theme::context_menu_item(ui, &t, theme::paint_select_all_icon, "Tout selectionner", "Ctrl+A", true);
            });
        });
        out.textures_delta.clear();
    }

    #[test]
    fn test_inspect_list_rendering() {
        let t = Tokens::for_kind(crate::theme::ThemeKind::default());
        let ctx = egui::Context::default();
        let tmp = tempfile::tempdir().unwrap();
        let vault = Vault::open(tmp.path()).unwrap();
        let mut note_to_open = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let p_resp = ui.scope(|ui| {
                    render_inline_spans(ui, "Defis :", &mut note_to_open, false, &vault, &t);
                });
                println!("Paragraph scope rect: {:?}", p_resp.response.rect);

                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let (marker_rect, _) = ui.allocate_exact_size(vec2(12.0, 20.0), Sense::hover());
                    let center = pos2(marker_rect.left() + 5.0, marker_rect.top() + 11.5);
                    println!("marker_rect: {:?}, center: {:?}", marker_rect, center);

                    let text_width = ui.available_width();
                    let t_resp = ui.allocate_ui_with_layout(vec2(text_width, 20.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        render_inline_spans(ui, "Parle fort", &mut note_to_open, false, &vault, &t);
                    });
                    println!("allocate_ui_with_layout rect: {:?}", t_resp.response.rect);
                });
            });
        });
        out.textures_delta.clear();
    }
}
