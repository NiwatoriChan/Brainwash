//! Application state and actions for MyBrain.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::{Datelike, Days, Local, NaiveDate};
use egui::Color32;
use mybrain_core::{
    AppSettings, CanvasFile, DailyNotes, EventDraft, EventStore, LinkIndex, NoteId, QuickNotes,
    Vault,
};
use serde::{Deserialize, Serialize};

/// How long a transient status message stays visible.
pub const STATUS_TTL: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    EditOnly,
    Split,
    PreviewOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopBarMode {
    Notes,
    Calendar,
    Graph,
    Canvas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidebarSection {
    Notes,
    Daily,
    Events,
    Quick,
}


/// Payload stored in egui drag-and-drop when dragging a note or canvas from the sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraggedNote {
    pub id: String,
    pub name: String,
    pub is_canvas: bool,
}

pub struct AppState {
    pub vault: Vault,
    pub link_index: LinkIndex,
    pub daily_config: DailyNotes,
    pub quick_config: QuickNotes,
    pub event_store: EventStore,

    // Active note, open tabs & Editor
    pub active_note: Option<NoteId>,
    pub open_tabs: Vec<NoteId>,
    pub editor_buffer: String,
    pub editor_cursor: usize,
    pub editor_dirty: bool,
    pub view_mode: ViewMode,

    // History for navigation
    pub history: Vec<NoteId>,
    pub history_idx: usize,

    // Tabs & Panels
    pub top_mode: TopBarMode,
    pub sidebar_section: SidebarSection,
    pub show_sidebar: bool,
    pub show_backlinks: bool,
    pub search_query: String,

    // Graph state
    pub graph: crate::views::graph_view::GraphState,

    // Canvas State & Document
    pub active_canvas_path: Option<String>,
    pub canvas_doc: Option<mybrain_core::CanvasFile>,
    pub canvas_dirty: bool,
    pub canvas_state: crate::views::canvas_view::CanvasState,

    // Settings
    pub settings: mybrain_core::AppSettings,
    pub settings_path: PathBuf,
    pub show_settings_dialog: bool,

    // Vault Switcher
    pub show_vault_dialog: bool,
    pub vault_path_input: String,

    // In-Folder Creation
    pub new_note_folder: Option<String>,
    pub new_event_folder: Option<String>,
    pub show_new_folder_dialog: bool,
    pub new_folder_parent: Option<String>,
    pub new_folder_name: String,
    pub show_new_canvas_dialog: bool,
    pub new_canvas_folder: Option<String>,
    pub new_canvas_name: String,

    // Modals / Overlays
    pub show_command_palette: bool,
    pub command_palette_query: String,
    pub command_palette_selected: usize,

    pub show_quick_capture: bool,
    pub quick_capture_buffer: String,

    // Quick Notes (Google Keep mode)
    pub quick_composer_title: String,
    pub quick_composer_expanded: bool,
    pub quick_composer_pinned: bool,
    pub quick_composer_color: Option<String>,
    pub quick_search_query: String,

    // In-place Keep note edit dialog
    pub quick_edit_note: Option<NoteId>,
    pub quick_edit_title: String,
    pub quick_edit_body: String,
    pub quick_edit_pinned: bool,
    pub quick_edit_color: Option<String>,

    pub show_new_note_dialog: bool,
    pub new_note_name: String,

    pub show_new_event_dialog: bool,
    pub new_event_draft: EventDraft,

    // Insert Table Dialog
    pub show_insert_table_dialog: bool,
    pub insert_table_cols: usize,
    pub insert_table_rows: usize,

    // Insert Image Dialog
    pub show_insert_image_dialog: bool,
    pub insert_image_alt: String,
    pub insert_image_url: String,

    // Daily notes navigation date & Calendar view
    pub selected_daily_date: NaiveDate,
    pub calendar_view_date: NaiveDate,

    // Theme
    pub theme_kind: crate::theme::ThemeKind,

    // Transient UI toast/status
    pub status: Option<(String, Instant)>,
}

impl AppState {
    pub fn new(vault_path: PathBuf) -> Result<Self, mybrain_core::Error> {
        let mut vault = Vault::open(&vault_path)?;
        let _ = LinkIndex::build(&vault);
        let daily_config = DailyNotes::default();
        let quick_config = QuickNotes::default();
        let event_store = EventStore::default();
        let today = Local::now().date_naive();

        // If vault is completely empty, create a welcome note!
        if vault.is_empty() {
            let welcome_id = NoteId::new("Welcome to Brainwash")?;
            let welcome_content = r#"# Welcome to Brainwash

Brainwash is a local-first, distraction-free **knowledge base** powered by Rust and egui.

## Features
- **Double-brackets [[Wiki Links]]**: Connect notes seamlessly like Obsidian. Try clicking [[Daily Notes]] or [[Quick Notes]]!
- **Daily Notes**: One note for each day with automatic templates.
- **Quick Capture**: Press the Quick Capture button or shortcut to record fleeting thoughts instantly.
- **Calendar Events**: Manage dates and appointments right inside plain Markdown files.
- **Split & Live Preview**: Switch between Edit, Split, and rendered Markdown views.
- **Backlinks Panel**: View incoming and outgoing connections for any note.

### Getting Started
- Press **Ctrl+P** or click **Search** to open the Command Palette.
- Press **Ctrl+Q** to open Quick Capture.
- Type `[[` inside any note to link to another page!
"#;
            let _ = vault.create(welcome_id.clone(), welcome_content);
        }

        let link_index = LinkIndex::build(&vault);
        let settings_path = dirs::config_dir()
            .map(|p| p.join("brainwash").join("settings.json"))
            .unwrap_or_else(|| vault_path.join(".brainwash_settings.json"));
        let mut settings = AppSettings::load_from_file(&settings_path);
        settings.record_recent_vault(&vault_path.to_string_lossy());
        let _ = settings.save_to_file(&settings_path);

        let theme_kind = match settings.theme.as_str() {
            "StudioLight" => crate::theme::ThemeKind::StudioLight,
            "Pro" => crate::theme::ThemeKind::Pro,
            "SolarizedDark" => crate::theme::ThemeKind::SolarizedDark,
            "AdwaitaDark" => crate::theme::ThemeKind::AdwaitaDark,
            _ => crate::theme::ThemeKind::Studio,
        };

        let mut state = Self {
            vault,
            link_index,
            daily_config,
            quick_config,
            event_store,
            active_note: None,
            open_tabs: Vec::new(),
            editor_buffer: String::new(),
            editor_cursor: 0,
            editor_dirty: false,
            view_mode: ViewMode::Split,
            history: Vec::new(),
            history_idx: 0,
            top_mode: TopBarMode::Notes,
            sidebar_section: SidebarSection::Notes,
            show_sidebar: true,
            show_backlinks: false,
            search_query: String::new(),
            graph: crate::views::graph_view::GraphState::default(),
            active_canvas_path: None,
            canvas_doc: None,
            canvas_dirty: false,
            canvas_state: crate::views::canvas_view::CanvasState::default(),
            settings,
            settings_path,
            show_settings_dialog: false,
            show_vault_dialog: false,
            vault_path_input: String::new(),
            new_note_folder: None,
            new_event_folder: None,
            show_new_folder_dialog: false,
            new_folder_parent: None,
            new_folder_name: String::new(),
            show_new_canvas_dialog: false,
            new_canvas_folder: None,
            new_canvas_name: String::new(),
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
            show_quick_capture: false,
            quick_capture_buffer: String::new(),
            quick_composer_title: String::new(),
            quick_composer_expanded: false,
            quick_composer_pinned: false,
            quick_composer_color: None,
            quick_search_query: String::new(),
            quick_edit_note: None,
            quick_edit_title: String::new(),
            quick_edit_body: String::new(),
            quick_edit_pinned: false,
            quick_edit_color: None,
            show_new_note_dialog: false,
            new_note_name: String::new(),
            show_new_event_dialog: false,
            new_event_draft: EventDraft::new(today),
            show_insert_table_dialog: false,
            insert_table_cols: 3,
            insert_table_rows: 3,
            show_insert_image_dialog: false,
            insert_image_alt: String::new(),
            insert_image_url: String::new(),
            selected_daily_date: today,
            calendar_view_date: today,
            theme_kind,
            status: Some(("Welcome to Brainwash".to_string(), Instant::now())),
        };

        let first_id = state.vault.notes().next().map(|n| n.id.clone());
        if let Some(id) = first_id {
            state.open_note(id);
        }

        Ok(state)
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    /// The current status message, if it hasn't expired yet.
    pub fn active_status(&self) -> Option<&str> {
        self.status
            .as_ref()
            .filter(|(_, at)| at.elapsed() < STATUS_TTL)
            .map(|(msg, _)| msg.as_str())
    }

    pub fn save_current(&mut self) {
        if !self.editor_dirty {
            return;
        }
        if let Some(id) = &self.active_note {
            if let Err(e) = self.vault.save(id, &self.editor_buffer) {
                self.set_status(format!("Error saving note: {e}"));
            } else {
                self.editor_dirty = false;
                self.link_index = LinkIndex::build(&self.vault);
                self.set_status("Note saved");
            }
        }
    }

    pub fn open_note(&mut self, id: NoteId) {
        if !self.open_tabs.contains(&id) {
            self.open_tabs.push(id.clone());
        }
        if self.active_note.as_ref() == Some(&id) {
            self.top_mode = TopBarMode::Notes;
            self.sidebar_section = SidebarSection::Notes;
            return;
        }
        self.save_current();

        if let Some(note) = self.vault.get(&id) {
            self.editor_buffer = note.content.clone();
            self.editor_cursor = self.editor_buffer.len();
            self.editor_dirty = false;
            self.active_note = Some(id.clone());

            // History tracking
            if self.history_idx < self.history.len() {
                self.history.truncate(self.history_idx + 1);
            }
            self.history.push(id);
            self.history_idx = self.history.len() - 1;
            self.top_mode = TopBarMode::Notes;
            self.sidebar_section = SidebarSection::Notes;
        }
    }

    pub fn close_tab(&mut self, id: &NoteId) {
        let was_active = self.active_note.as_ref() == Some(id);
        if was_active {
            self.save_current();
        }
        if let Some(pos) = self.open_tabs.iter().position(|t| t == id) {
            self.open_tabs.remove(pos);
            if was_active {
                if !self.open_tabs.is_empty() {
                    let next_idx = pos.min(self.open_tabs.len() - 1);
                    let next_id = self.open_tabs[next_idx].clone();
                    if let Some(note) = self.vault.get(&next_id) {
                        self.editor_buffer = note.content.clone();
                        self.editor_cursor = self.editor_buffer.len();
                        self.editor_dirty = false;
                        self.active_note = Some(next_id);
                    } else {
                        self.active_note = None;
                        self.editor_buffer.clear();
                        self.editor_dirty = false;
                    }
                } else {
                    self.active_note = None;
                    self.editor_buffer.clear();
                    self.editor_dirty = false;
                }
            }
        }
    }

    pub fn close_current_tab(&mut self) {
        if let Some(id) = self.active_note.clone() {
            self.close_tab(&id);
        }
    }

    pub fn cycle_next_tab(&mut self) {
        if self.open_tabs.len() <= 1 {
            return;
        }
        if let Some(curr) = &self.active_note {
            if let Some(pos) = self.open_tabs.iter().position(|t| t == curr) {
                let next_pos = (pos + 1) % self.open_tabs.len();
                let next_id = self.open_tabs[next_pos].clone();
                self.open_note(next_id);
            }
        }
    }

    pub fn open_or_create(&mut self, raw_target: &str) {
        if let Some(target_id) = self.vault.resolve_link(raw_target) {
            self.open_note(target_id.clone());
        } else if let Ok(new_id) = NoteId::new(raw_target) {
            if let Some(folder) = new_id.folder() {
                if self.is_reserved_special_folder(folder) {
                    self.set_status("Cannot create normal notes in Daily or Quick Notes folders.");
                    return;
                }
            }
            let title = new_id.name();
            let template = format!("# {title}\n\n");
            match self.vault.create(new_id.clone(), &template) {
                Ok(_) => {
                    self.link_index = LinkIndex::build(&self.vault);
                    self.open_note(new_id);
                    self.set_status(format!("Created note [[{raw_target}]]"));
                }
                Err(e) => {
                    self.set_status(format!("Failed to create note: {e}"));
                }
            }
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.history_idx > 0 && !self.history.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.history.is_empty() && self.history_idx + 1 < self.history.len()
    }

    pub fn go_back(&mut self) {
        if self.can_go_back() {
            self.save_current();
            self.history_idx -= 1;
            if let Some(id) = self.history.get(self.history_idx).cloned() {
                if let Some(note) = self.vault.get(&id) {
                    self.editor_buffer = note.content.clone();
                    self.editor_dirty = false;
                    self.active_note = Some(id);
                }
            }
        }
    }

    pub fn go_forward(&mut self) {
        if self.can_go_forward() {
            self.save_current();
            self.history_idx += 1;
            if let Some(id) = self.history.get(self.history_idx).cloned() {
                if let Some(note) = self.vault.get(&id) {
                    self.editor_buffer = note.content.clone();
                    self.editor_dirty = false;
                    self.active_note = Some(id);
                }
            }
        }
    }

    pub fn is_daily_note(&self, id: &NoteId) -> bool {
        self.daily_config.date_of(id).is_some()
            || id.folder().map(|f| f.eq_ignore_ascii_case(&self.daily_config.folder) || f.to_lowercase().starts_with("daily/")).unwrap_or(false)
            || id.as_str().to_lowercase().starts_with("daily/")
    }

    pub fn is_event_note(&self, id: &NoteId) -> bool {
        self.vault.get(id).map(|n| mybrain_core::Event::from_note(n).is_some()).unwrap_or(false)
            || id.folder().map(|f| f.eq_ignore_ascii_case(&self.event_store.folder) || f.to_lowercase().starts_with("events/")).unwrap_or(false)
            || id.as_str().to_lowercase().starts_with("events/")
    }

    pub fn is_daily_or_event_folder(&self, folder: &str) -> bool {
        let clean = folder.trim().trim_matches('/');
        let daily = self.daily_config.folder.trim().trim_matches('/');
        let events = self.event_store.folder.trim().trim_matches('/');

        clean.eq_ignore_ascii_case(daily)
            || clean.to_lowercase().starts_with(&format!("{}/", daily.to_lowercase()))
            || clean.eq_ignore_ascii_case(events)
            || clean.to_lowercase().starts_with(&format!("{}/", events.to_lowercase()))
            || clean.eq_ignore_ascii_case("daily")
            || clean.to_lowercase().starts_with("daily/")
            || clean.eq_ignore_ascii_case("events")
            || clean.to_lowercase().starts_with("events/")
    }

    pub fn is_reserved_special_folder(&self, folder: &str) -> bool {
        let clean = folder.trim().trim_matches('/');
        let daily = self.daily_config.folder.trim().trim_matches('/');
        let events = self.event_store.folder.trim().trim_matches('/');
        let quick = self.quick_config.folder.trim().trim_matches('/');

        clean.eq_ignore_ascii_case(daily)
            || clean.to_lowercase().starts_with(&format!("{}/", daily.to_lowercase()))
            || clean.eq_ignore_ascii_case(events)
            || clean.to_lowercase().starts_with(&format!("{}/", events.to_lowercase()))
            || clean.eq_ignore_ascii_case(quick)
            || clean.to_lowercase().starts_with(&format!("{}/", quick.to_lowercase()))
            || clean.eq_ignore_ascii_case("quick")
            || clean.to_lowercase().starts_with("quick/")
            || clean.eq_ignore_ascii_case("events")
            || clean.to_lowercase().starts_with("events/")
    }

    pub fn get_note_color(&self, id: &NoteId) -> Option<Color32> {
        let content = if self.active_note.as_ref() == Some(id) {
            &self.editor_buffer
        } else {
            &self.vault.get(id)?.content
        };
        let (fm, _) = mybrain_core::Frontmatter::parse(content);
        let color_val = fm.get("color")?;
        parse_color_str(color_val)
    }

    pub fn set_note_color(&mut self, id: &NoteId, color: Option<&str>) {
        let is_active = self.active_note.as_ref() == Some(id);
        let content = if is_active {
            self.editor_buffer.clone()
        } else if let Some(n) = self.vault.get(id) {
            n.content.clone()
        } else {
            return;
        };

        let (mut fm, body) = mybrain_core::Frontmatter::parse(&content);
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

        let _ = self.vault.save(id, &new_content);
        if is_active {
            self.editor_buffer = new_content;
            self.editor_dirty = false;
        }
    }

    pub fn get_note_emoji(&self, id: &NoteId) -> Option<String> {
        let content = if self.active_note.as_ref() == Some(id) {
            &self.editor_buffer
        } else if let Some(n) = self.vault.get(id) {
            &n.content
        } else {
            return extract_leading_emoji(id.name());
        };
        let (fm, _) = mybrain_core::Frontmatter::parse(content);
        if let Some(icon) = fm.get("icon") {
            let trimmed = icon.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        extract_leading_emoji(id.name())
    }

    pub fn set_note_emoji(&mut self, id: &NoteId, emoji: Option<&str>) {
        let is_active = self.active_note.as_ref() == Some(id);
        let content = if is_active {
            self.editor_buffer.clone()
        } else if let Some(n) = self.vault.get(id) {
            n.content.clone()
        } else {
            return;
        };

        let (mut fm, body) = mybrain_core::Frontmatter::parse(&content);
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

        let _ = self.vault.save(id, &new_content);
        if is_active {
            self.editor_buffer = new_content;
            self.editor_dirty = false;
        }
    }

    pub fn get_folder_emoji(&self, folder_path: &str) -> Option<String> {
        let clean = folder_path.trim().trim_matches('/');
        if let Some(emoji) = self.settings.folder_emojis.get(clean) {
            return Some(emoji.clone());
        }
        let folder_name = clean.rsplit('/').next().unwrap_or(clean);
        extract_leading_emoji(folder_name)
    }

    pub fn set_folder_emoji(&mut self, folder_path: &str, emoji: Option<&str>) {
        let clean = folder_path.trim().trim_matches('/').to_string();
        if let Some(e) = emoji {
            self.settings.folder_emojis.insert(clean, e.trim().to_string());
        } else {
            self.settings.folder_emojis.remove(&clean);
        }
        let settings_path = self.vault.root().join(".brainwash").join("settings.json");
        let _ = self.settings.save_to_file(&settings_path);
    }

    pub fn create_note(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        match NoteId::new(name) {
            Ok(id) => {
                if let Some(folder) = id.folder() {
                    if self.is_reserved_special_folder(folder) {
                        self.set_status("Cannot add normal notes in Daily or Quick Notes folders.");
                        return;
                    }
                }
                let initial = format!("# {}\n\n", id.name());
                match self.vault.create(id.clone(), &initial) {
                    Ok(_) => {
                        self.link_index = LinkIndex::build(&self.vault);
                        self.open_note(id);
                        self.set_status(format!("Created {}", name));
                        self.show_new_note_dialog = false;
                        self.new_note_name.clear();
                    }
                    Err(e) => self.set_status(format!("Cannot create note: {e}")),
                }
            }
            Err(e) => self.set_status(format!("Invalid name: {e}")),
        }
    }

    pub fn delete_current_note(&mut self) {
        if let Some(id) = self.active_note.take() {
            self.open_tabs.retain(|t| t != &id);
            if let Err(e) = self.vault.delete(&id) {
                self.set_status(format!("Error deleting: {e}"));
            } else {
                self.editor_buffer.clear();
                self.editor_dirty = false;
                self.link_index = LinkIndex::build(&self.vault);
                self.set_status("Note deleted");
                if let Some(next_tab) = self.open_tabs.first().cloned() {
                    self.open_note(next_tab);
                } else {
                    let next_id = self.vault.notes().next().map(|n| n.id.clone());
                    if let Some(id) = next_id {
                        self.open_note(id);
                    }
                }
            }
        }
    }

    pub fn open_daily_today(&mut self) {
        let today = Local::now().date_naive();
        self.open_daily_date(today);
    }

    pub fn open_daily_date(&mut self, date: NaiveDate) {
        self.selected_daily_date = date;
        let id = self.daily_config.id_for(date);
        if !self.vault.contains(&id) {
            let day_events = self.event_store.on(&self.vault, date);
            let events_text = if day_events.is_empty() {
                None
            } else {
                let mut buf = String::new();
                for ev in &day_events {
                    buf.push_str(&format!("- [ ] **{}** [[{}]]\n", ev.time_label(), ev.id.name()));
                }
                Some(buf)
            };
            let content = self.daily_config.render_template_with_events(date, events_text.as_deref());
            if let Err(e) = self.vault.create(id.clone(), &content) {
                self.set_status(format!("Failed to create daily note: {e}"));
                return;
            }
        }

        self.link_index = LinkIndex::build(&self.vault);
        self.open_note(id);
        self.top_mode = TopBarMode::Notes;
        self.set_status(format!("Opened daily note for {}", date));
    }

    pub fn shift_daily_date(&mut self, days: i64) {
        let new_date = if days >= 0 {
            self.selected_daily_date.checked_add_days(Days::new(days as u64))
        } else {
            self.selected_daily_date.checked_sub_days(Days::new((-days) as u64))
        };
        if let Some(d) = new_date {
            self.open_daily_date(d);
        }
    }

    #[allow(dead_code)]
    pub fn capture_quick_note(&mut self) {
        let text = self.quick_capture_buffer.trim().to_string();
        if text.is_empty() {
            return;
        }
        let now = Local::now().naive_local();
        match self.quick_config.capture(&mut self.vault, &self.daily_config, &text, now) {
            Ok(id) => {
                self.link_index = LinkIndex::build(&self.vault);
                self.quick_capture_buffer.clear();
                self.show_quick_capture = false;
                self.set_status("Quick note saved!");
                self.open_note(id);
            }
            Err(e) => self.set_status(format!("Failed quick capture: {e}")),
        }
    }

    pub fn capture_quick_keep(&mut self) {
        let title = self.quick_composer_title.trim();
        let text = self.quick_capture_buffer.trim();
        if title.is_empty() && text.is_empty() {
            return;
        }
        let now = Local::now().naive_local();
        let title_opt = if title.is_empty() { None } else { Some(title) };
        let color_opt = self.quick_composer_color.as_deref();
        let pinned = self.quick_composer_pinned;

        match self.quick_config.capture_full(
            &mut self.vault,
            &self.daily_config,
            title_opt,
            text,
            pinned,
            color_opt,
            now,
        ) {
            Ok(id) => {
                self.link_index = LinkIndex::build(&self.vault);
                self.quick_composer_title.clear();
                self.quick_capture_buffer.clear();
                self.quick_composer_pinned = false;
                self.quick_composer_color = None;
                self.quick_composer_expanded = false;
                self.show_quick_capture = false;
                self.set_status("Quick note saved!");
                let _ = id;
            }
            Err(e) => self.set_status(format!("Failed quick capture: {e}")),
        }
    }

    pub fn open_quick_edit(&mut self, id: NoteId) {
        if let Some(note) = self.vault.get(&id) {
            let fm = note.frontmatter();
            let title = fm.get("title").map(|s| s.to_string()).unwrap_or_else(|| note.title().to_string());
            let pinned = fm.get("pinned") == Some("true");
            let color = fm.get("color").map(|s| s.to_string());
            let raw_body = note.body();
            let body = raw_body
                .trim_start_matches(|c: char| c == '#' || c.is_whitespace() || c == '\n')
                .to_string();

            self.quick_edit_note = Some(id);
            self.quick_edit_title = title;
            self.quick_edit_body = body;
            self.quick_edit_pinned = pinned;
            self.quick_edit_color = color;
        }
    }

    pub fn save_quick_edit(&mut self) {
        if let Some(id) = self.quick_edit_note.take() {
            if let Some(note) = self.vault.get(&id) {
                let mut fm = note.frontmatter().clone();
                let clean_title = self.quick_edit_title.trim();
                if !clean_title.is_empty() {
                    fm.set("title", clean_title);
                }
                if self.quick_edit_pinned {
                    fm.set("pinned", "true");
                } else {
                    fm.remove("pinned");
                }
                if let Some(c) = &self.quick_edit_color {
                    if !c.is_empty() {
                        fm.set("color", c);
                    } else {
                        fm.remove("color");
                    }
                } else {
                    fm.remove("color");
                }

                let new_content = if !clean_title.is_empty() {
                    format!("{}# {}\n\n{}\n", fm.render(), clean_title, self.quick_edit_body.trim())
                } else {
                    format!("{}{}\n", fm.render(), self.quick_edit_body.trim())
                };

                if let Err(e) = self.vault.save(&id, &new_content) {
                    self.set_status(format!("Error saving quick note: {e}"));
                } else {
                    self.link_index = LinkIndex::build(&self.vault);
                    self.set_status("Quick note updated");
                }
            }
        }
    }

    pub fn toggle_quick_pinned(&mut self, id: &NoteId) {
        if let Some(note) = self.vault.get(id) {
            let mut fm = note.frontmatter().clone();
            let is_pinned = fm.get("pinned") == Some("true");
            if is_pinned {
                fm.remove("pinned");
            } else {
                fm.set("pinned", "true");
            }
            let new_content = format!("{}{}", fm.render(), note.body());
            let _ = self.vault.save(id, &new_content);
        }
    }

    pub fn set_quick_color(&mut self, id: &NoteId, color: Option<&str>) {
        if let Some(note) = self.vault.get(id) {
            let mut fm = note.frontmatter().clone();
            if let Some(c) = color {
                fm.set("color", c);
            } else {
                fm.remove("color");
            }
            let new_content = format!("{}{}", fm.render(), note.body());
            let _ = self.vault.save(id, &new_content);
        }
    }

    pub fn delete_quick_note(&mut self, id: &NoteId) {
        if let Err(e) = self.vault.delete(id) {
            self.set_status(format!("Error deleting quick note: {e}"));
        } else {
            self.link_index = LinkIndex::build(&self.vault);
            if self.active_note.as_ref() == Some(id) {
                self.active_note = None;
                self.editor_buffer.clear();
            }
            self.set_status("Quick note deleted");
        }
    }

    pub fn insert_snippet_into_active_note(&mut self, snippet: &str) {
        if self.active_note.is_none() {
            self.set_status("No active note open to insert into.");
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
        self.set_status("Inserted object into note");
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

    pub fn create_event(&mut self) {
        let draft = self.new_event_draft.clone();
        if let Err(e) = draft.validate() {
            self.set_status(format!("Invalid event: {e}"));
            return;
        }

        let event_block = draft.render_block();

        // Target note: if user is currently editing a note, insert into that note.
        // Otherwise, insert into that date's daily note.
        let target_id = if let Some(active) = &self.active_note {
            active.clone()
        } else {
            match self.daily_config.open_or_create(&mut self.vault, draft.date) {
                Ok(id) => id,
                Err(e) => {
                    self.set_status(format!("Failed to open daily note: {e}"));
                    return;
                }
            }
        };

        if self.active_note.as_ref() == Some(&target_id) {
            self.insert_snippet_into_active_note(&format!("\n{event_block}\n"));
            self.save_current();
        } else {
            let _ = self.vault.append(&target_id, &format!("\n{event_block}\n"));
            self.open_note(target_id.clone());
        }

        self.link_index = LinkIndex::build(&self.vault);
        self.show_new_event_dialog = false;
        self.set_status(format!("Added event object to [[{}]]", target_id.name()));
    }

    /// Appends or synchronizes scheduled events for `date` into that day's daily note.
    pub fn sync_events_to_daily_note(&mut self, date: NaiveDate) {
        let events = self.event_store.on(&self.vault, date);
        if events.is_empty() {
            self.set_status("No events scheduled for this date.");
            return;
        }

        let daily_id = match self.daily_config.open_or_create(&mut self.vault, date) {
            Ok(id) => id,
            Err(e) => {
                self.set_status(format!("Failed to open daily note: {e}"));
                return;
            }
        };

        if let Some(daily_note) = self.vault.get(&daily_id) {
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
                let _ = self.vault.save(&daily_id, &content);
                self.link_index = LinkIndex::build(&self.vault);
                if self.active_note.as_ref() == Some(&daily_id) {
                    self.editor_buffer = content;
                    self.editor_dirty = false;
                }
                self.set_status(format!("Synced {added} event(s) to daily note"));
            } else {
                self.set_status("All events for this day are already listed in the note");
            }
        }
    }

    pub fn save_settings(&self) {
        let _ = self.settings.save_to_file(&self.settings_path);
    }

    pub fn switch_vault(&mut self, new_root: PathBuf) -> Result<(), mybrain_core::Error> {
        let mut new_vault = Vault::open(&new_root)?;
        if new_vault.is_empty() {
            let welcome_id = NoteId::new("Welcome to Brainwash")?;
            let _ = new_vault.create(welcome_id.clone(), "# Welcome to Brainwash\nYour fresh new vault.");
        }
        self.link_index = LinkIndex::build(&new_vault);
        self.vault = new_vault;
        self.active_note = None;
        self.editor_buffer.clear();
        self.editor_dirty = false;
        self.history.clear();
        self.history_idx = 0;
        self.active_canvas_path = None;
        self.canvas_doc = None;
        self.canvas_dirty = false;
        self.settings.record_recent_vault(&self.vault.root().to_string_lossy());
        self.save_settings();

        let first = self.vault.notes().next().map(|n| n.id.clone());
        if let Some(id) = first {
            self.open_note(id);
        }
        self.set_status(format!("Opened vault: {}", self.vault.root().display()));
        Ok(())
    }

    pub fn open_canvas(&mut self, rel_path: &str) {
        match self.vault.load_canvas(rel_path) {
            Ok(doc) => {
                self.active_canvas_path = Some(rel_path.to_owned());
                self.canvas_doc = Some(doc);
                self.canvas_dirty = false;
                self.top_mode = TopBarMode::Canvas;
                self.set_status(format!("Opened canvas: {rel_path}"));
            }
            Err(e) => self.set_status(format!("Failed to open canvas: {e}")),
        }
    }

    pub fn save_canvas(&mut self) {
        if let (Some(path), Some(doc)) = (&self.active_canvas_path, &self.canvas_doc) {
            if let Err(e) = self.vault.save_canvas(path, doc) {
                self.set_status(format!("Failed to save canvas: {e}"));
            } else {
                self.canvas_dirty = false;
            }
        }
    }

    pub fn create_canvas(&mut self, folder: Option<&str>, name: &str) -> Result<String, mybrain_core::Error> {
        let clean_name = name.trim().trim_end_matches(".canvas");
        let safe_name = if clean_name.is_empty() { "Untitled" } else { clean_name };
        let rel_path = match folder {
            Some(f) if !f.is_empty() => format!("{f}/{safe_name}.canvas"),
            _ => format!("{safe_name}.canvas"),
        };
        let mut doc = CanvasFile::new();
        doc.add_text_node("New Idea\nType Markdown or [[Links]] here…", 0.0, 0.0, 220.0, 140.0);
        self.vault.save_canvas(&rel_path, &doc)?;
        self.open_canvas(&rel_path);
        Ok(rel_path)
    }

    pub fn create_folder(&mut self, parent: Option<&str>, name: &str) -> Result<(), mybrain_core::Error> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err(mybrain_core::Error::InvalidName("Folder name cannot be empty".into()));
        }
        if let Some(p) = parent {
            if self.is_reserved_special_folder(p) {
                return Err(mybrain_core::Error::InvalidName("Cannot create subfolders inside Daily or Quick Notes".into()));
            }
        }
        let rel = match parent {
            Some(p) if !p.is_empty() => format!("{p}/{clean}"),
            _ => clean.to_owned(),
        };
        if self.is_reserved_special_folder(&rel) {
            return Err(mybrain_core::Error::InvalidName("Cannot create folder with reserved name (Daily, Quick Notes)".into()));
        }
        self.vault.create_folder(&rel)?;
        self.set_status(format!("Created folder: {rel}"));
        Ok(())
    }

    pub fn shift_calendar_month(&mut self, delta: i32) {
        let year = self.calendar_view_date.year();
        let month = self.calendar_view_date.month() as i32;
        let total_months = year * 12 + (month - 1) + delta;
        let new_year = total_months.div_euclid(12);
        let new_month = (total_months.rem_euclid(12) + 1) as u32;
        if let Some(d) = NaiveDate::from_ymd_opt(new_year, new_month, 1) {
            self.calendar_view_date = d;
        }
    }
}

/// Parse a color string (either hex `#RRGGBB` / `#RGB` or named preset from QUICK_COLORS).
pub fn parse_color_str(color_str: &str) -> Option<Color32> {
    let s = color_str.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("default") {
        return None;
    }
    for def in crate::views::quick_capture::QUICK_COLORS {
        if def.name.eq_ignore_ascii_case(s) {
            return Some(def.dot_color);
        }
    }
    if let Some(hex) = s.strip_prefix('#') {
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
    }
    None
}

/// Extract leading emoji symbol from a string if present.
pub fn extract_leading_emoji(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let first = chars.next()?;
    if (first >= '\u{1F300}' && first <= '\u{1FAFF}')
        || (first >= '\u{2600}' && first <= '\u{27BF}')
        || (first >= '\u{2300}' && first <= '\u{23FF}')
        || (first >= '\u{2B50}' && first <= '\u{2B55}')
    {
        return Some(first.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_app_state() -> (tempfile::TempDir, AppState) {
        let tmp = tempfile::tempdir().unwrap();
        let state = AppState::new(tmp.path().to_path_buf()).unwrap();
        (tmp, state)
    }

    #[test]
    fn test_is_reserved_special_folder() {
        let (_tmp, state) = temp_app_state();
        assert!(state.is_reserved_special_folder("Daily"));
        assert!(state.is_reserved_special_folder("daily"));
        assert!(state.is_reserved_special_folder("Daily/2026"));
        assert!(state.is_reserved_special_folder("Quick Notes"));
        assert!(state.is_reserved_special_folder("quick notes"));
        assert!(state.is_reserved_special_folder("Quick Notes/Archive"));
        assert!(state.is_reserved_special_folder("quick"));
        assert!(state.is_reserved_special_folder("quick/foo"));

        assert!(!state.is_reserved_special_folder("Projects"));
        assert!(!state.is_reserved_special_folder("Work/DailyTasks"));
        assert!(!state.is_reserved_special_folder("Notes"));
    }

    #[test]
    fn test_cannot_create_normal_notes_in_reserved_folders() {
        let (_tmp, mut state) = temp_app_state();
        state.create_note("Daily/MyNote");
        assert!(state.vault.get(&NoteId::new("Daily/MyNote").unwrap()).is_none());
        assert!(state.active_status().unwrap().contains("Cannot add normal notes"));

        state.create_note("Quick Notes/Test");
        assert!(state.vault.get(&NoteId::new("Quick Notes/Test").unwrap()).is_none());
        assert!(state.active_status().unwrap().contains("Cannot add normal notes"));

        // But normal notes work
        state.create_note("Projects/MyNote");
        assert!(state.vault.get(&NoteId::new("Projects/MyNote").unwrap()).is_some());
    }

    #[test]
    fn test_cannot_create_folders_in_reserved_folders() {
        let (_tmp, mut state) = temp_app_state();
        assert!(state.create_folder(Some("Daily"), "Sub").is_err());
        assert!(state.create_folder(None, "Daily").is_err());
        assert!(state.create_folder(None, "Quick Notes").is_err());

        assert!(state.create_folder(None, "Projects").is_ok());
        assert!(state.create_folder(Some("Projects"), "Sub").is_ok());
    }

    #[test]
    fn test_open_note_switches_to_text_editor() {
        let (_tmp, mut state) = temp_app_state();
        state.create_note("TestNote");
        let id = NoteId::new("TestNote").unwrap();

        // Switch to daily section and canvas top mode
        state.sidebar_section = SidebarSection::Daily;
        state.top_mode = TopBarMode::Canvas;

        // Calling open_note even if active switches to Notes text editor
        state.open_note(id.clone());
        assert_eq!(state.sidebar_section, SidebarSection::Notes);
        assert_eq!(state.top_mode, TopBarMode::Notes);
        assert_eq!(state.active_note.as_ref(), Some(&id));
    }

    #[test]
    fn test_create_event_integrates_into_daily_note() {
        let (_tmp, mut state) = temp_app_state();
        let event_date = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        state.active_note = None;
        state.new_event_draft = EventDraft {
            title: "Client Meeting".into(),
            date: event_date,
            start: chrono::NaiveTime::from_hms_opt(14, 0, 0),
            end: chrono::NaiveTime::from_hms_opt(15, 0, 0),
            location: "Room 101".into(),
            description: "Discuss Q4 objectives.".into(),
        };
        state.create_event();

        // 1. Target daily note is open in editor
        let daily_id = state.daily_config.id_for(event_date);
        assert_eq!(state.active_note.as_ref(), Some(&daily_id));
        assert_eq!(state.sidebar_section, SidebarSection::Notes);

        // 2. Daily note created and has event object embedded
        let daily_note = state.vault.get(&daily_id).expect("daily note should be auto-created");
        assert!(daily_note.content.contains("Client Meeting"));
        assert!(daily_note.content.contains("14:00 - 15:00"));

        // 3. Event is discoverable by event_store across all notes
        let events = state.event_store.on(&state.vault, event_date);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Client Meeting");
        assert_eq!(events[0].location.as_deref(), Some("Room 101"));
    }

    #[test]
    fn test_create_event_embeds_into_active_note() {
        let (_tmp, mut state) = temp_app_state();
        let project_id = NoteId::new("Projects/Alpha").unwrap();
        state.vault.create(project_id.clone(), "# Project Alpha\n\nMeeting notes:\n").unwrap();
        state.open_note(project_id.clone());

        let event_date = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        state.new_event_draft = EventDraft {
            title: "Sprint Review".into(),
            date: event_date,
            start: chrono::NaiveTime::from_hms_opt(10, 0, 0),
            end: chrono::NaiveTime::from_hms_opt(11, 0, 0),
            location: "Room B".into(),
            description: "Review sprint deliverables.".into(),
        };
        state.create_event();

        // Embedded in active note
        assert_eq!(state.active_note.as_ref(), Some(&project_id));
        let note = state.vault.get(&project_id).unwrap();
        assert!(note.content.contains("Sprint Review"));
        assert!(note.content.contains("```event"));

        // Assembled in event_store
        let events = state.event_store.on(&state.vault, event_date);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Sprint Review");
        assert_eq!(events[0].id, project_id);
    }

    #[test]
    fn test_sync_events_to_daily_note() {
        let (_tmp, mut state) = temp_app_state();
        let event_date = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let draft = EventDraft {
            title: "Dentist".into(),
            date: event_date,
            start: chrono::NaiveTime::from_hms_opt(9, 30, 0),
            end: None,
            location: "".into(),
            description: "".into(),
        };
        // Add event to a project note
        let project_id = NoteId::new("Projects/Personal").unwrap();
        state.vault.create(project_id.clone(), "# Personal\n").unwrap();
        state.event_store.append_to_note(&mut state.vault, &project_id, &draft).unwrap();

        // Sync to daily note
        state.sync_events_to_daily_note(event_date);

        let daily_id = state.daily_config.id_for(event_date);
        let daily_note = state.vault.get(&daily_id).expect("daily note created");
        assert!(daily_note.content.contains("Dentist"));
        assert!(daily_note.content.contains("09:30"));
    }

    #[test]
    fn test_open_daily_date_creates_if_not_exists() {
        let (_tmp, mut state) = temp_app_state();
        let target_date = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
        let daily_id = state.daily_config.id_for(target_date);

        // Daily note does not exist yet
        assert!(state.vault.get(&daily_id).is_none());

        // Calling open_daily_date (as triggered when clicking any date on the calendar)
        state.open_daily_date(target_date);

        // Verify it was created and is active in editor
        assert!(state.vault.get(&daily_id).is_some());
        assert_eq!(state.active_note.as_ref(), Some(&daily_id));
        assert_eq!(state.top_mode, TopBarMode::Notes);
        assert_eq!(state.sidebar_section, SidebarSection::Notes);
        assert!(state.editor_buffer.contains("November 15, 2026"));
    }

    #[test]
    fn test_insert_table_and_image() {
        let (_tmp, mut state) = temp_app_state();
        let note_id = NoteId::new("Document").unwrap();
        state.vault.create(note_id.clone(), "# My Doc\n").unwrap();
        state.open_note(note_id);

        state.insert_table(2, 3);
        assert!(state.editor_buffer.contains("| Header 1 | Header 2 |"));
        assert!(state.editor_buffer.contains("| Cell 3-2 |"));

        state.insert_image("Architecture", "assets/arch.png");
        assert!(state.editor_buffer.contains("![Architecture](assets/arch.png)"));
    }

    #[test]
    fn test_note_custom_color_and_emoji() {
        let (_tmp, mut state) = temp_app_state();
        let note_id = NoteId::new("Projects/Idea").unwrap();
        state.vault.create(note_id.clone(), "# Idea\n").unwrap();
        state.open_note(note_id.clone());

        // Color initially None
        assert_eq!(state.get_note_color(&note_id), None);
        assert_eq!(state.get_note_emoji(&note_id), None);

        // Set preset color
        state.set_note_color(&note_id, Some("coral"));
        assert!(state.get_note_color(&note_id).is_some());
        assert!(state.editor_buffer.contains("color: coral"));

        // Set hex color
        state.set_note_color(&note_id, Some("#3a86ff"));
        assert_eq!(state.get_note_color(&note_id), Some(Color32::from_rgb(0x3a, 0x86, 0xff)));
        assert!(state.editor_buffer.contains("color: #3a86ff"));

        // Set emoji
        state.set_note_emoji(&note_id, Some("🚀"));
        assert_eq!(state.get_note_emoji(&note_id), Some("🚀".to_string()));
        assert!(state.editor_buffer.contains("icon: 🚀"));

        // Clear color and emoji
        state.set_note_color(&note_id, None);
        assert_eq!(state.get_note_color(&note_id), None);
        state.set_note_emoji(&note_id, None);
        assert_eq!(state.get_note_emoji(&note_id), None);
    }

    #[test]
    fn test_folder_emoji() {
        let (_tmp, mut state) = temp_app_state();
        assert_eq!(state.get_folder_emoji("Projects"), None);

        state.set_folder_emoji("Projects", Some("📁"));
        assert_eq!(state.get_folder_emoji("Projects"), Some("📁".to_string()));

        // Also works with trailing slashes
        assert_eq!(state.get_folder_emoji("Projects/"), Some("📁".to_string()));

        state.set_folder_emoji("Projects", None);
        assert_eq!(state.get_folder_emoji("Projects"), None);
    }

    #[test]
    fn test_is_daily_and_event_isolation() {
        let (_tmp, mut state) = temp_app_state();
        let daily_id = NoteId::new("Daily/2026-10-25").unwrap();
        let event_id = NoteId::new("Events/Meeting").unwrap();
        let normal_id = NoteId::new("Projects/Brainwash").unwrap();

        state.vault.create(daily_id.clone(), "# 2026-10-25\n").unwrap();
        state.vault.create(event_id.clone(), "---\ntype: event\ndate: 2026-10-25\n---\n# Meeting\n").unwrap();
        state.vault.create(normal_id.clone(), "# Brainwash\n").unwrap();

        assert!(state.is_daily_note(&daily_id));
        assert!(!state.is_daily_note(&normal_id));
        assert!(!state.is_daily_note(&event_id));

        assert!(state.is_event_note(&event_id));
        assert!(!state.is_event_note(&normal_id));

        assert!(state.is_daily_or_event_folder("Daily"));
        assert!(state.is_daily_or_event_folder("daily/2026"));
        assert!(state.is_daily_or_event_folder("Events"));
        assert!(!state.is_daily_or_event_folder("Projects"));
    }

    #[test]
    fn test_multiple_tabs() {
        let (_tmp, mut state) = temp_app_state();
        state.open_tabs.clear();
        state.active_note = None;
        let id1 = NoteId::new("Note1").unwrap();
        let id2 = NoteId::new("Note2").unwrap();
        let id3 = NoteId::new("Note3").unwrap();

        state.vault.create(id1.clone(), "# Note 1\n").unwrap();
        state.vault.create(id2.clone(), "# Note 2\n").unwrap();
        state.vault.create(id3.clone(), "# Note 3\n").unwrap();

        // Open Note 1
        state.open_note(id1.clone());
        assert_eq!(state.open_tabs, vec![id1.clone()]);
        assert_eq!(state.active_note.as_ref(), Some(&id1));

        // Open Note 2 -> added to tabs
        state.open_note(id2.clone());
        assert_eq!(state.open_tabs, vec![id1.clone(), id2.clone()]);
        assert_eq!(state.active_note.as_ref(), Some(&id2));

        // Open Note 3 -> added to tabs
        state.open_note(id3.clone());
        assert_eq!(state.open_tabs, vec![id1.clone(), id2.clone(), id3.clone()]);
        assert_eq!(state.active_note.as_ref(), Some(&id3));

        // Cycle next tab
        state.cycle_next_tab();
        assert_eq!(state.active_note.as_ref(), Some(&id1));

        // Close tab Note 2
        state.close_tab(&id2);
        assert_eq!(state.open_tabs, vec![id1.clone(), id3.clone()]);
        // Note 1 remains active
        assert_eq!(state.active_note.as_ref(), Some(&id1));

        // Close active tab (Note 1) -> switches to Note 3
        state.close_current_tab();
        assert_eq!(state.open_tabs, vec![id3.clone()]);
        assert_eq!(state.active_note.as_ref(), Some(&id3));

        // Close last tab -> empty tabs
        state.close_current_tab();
        assert!(state.open_tabs.is_empty());
        assert_eq!(state.active_note, None);
    }
}


