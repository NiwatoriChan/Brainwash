//! Application state and actions for MyBrain.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::{Datelike, Days, Local, NaiveDate};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DailyViewMode {
    MonthGrid,
    Agenda,
    Timeline,
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

    // Active note & Editor
    pub active_note: Option<NoteId>,
    pub editor_buffer: String,
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

    // Daily notes navigation date & Calendar view
    pub selected_daily_date: NaiveDate,
    pub daily_view_mode: DailyViewMode,
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
            editor_buffer: String::new(),
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
            selected_daily_date: today,
            daily_view_mode: DailyViewMode::MonthGrid,
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
        if self.active_note.as_ref() == Some(&id) {
            self.top_mode = TopBarMode::Notes;
            self.sidebar_section = SidebarSection::Notes;
            return;
        }
        self.save_current();

        if let Some(note) = self.vault.get(&id) {
            self.editor_buffer = note.content.clone();
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

    pub fn is_reserved_special_folder(&self, folder: &str) -> bool {
        let clean = folder.trim().trim_matches('/');
        let daily = self.daily_config.folder.trim().trim_matches('/');
        let quick = self.quick_config.folder.trim().trim_matches('/');

        clean.eq_ignore_ascii_case(daily)
            || clean.to_lowercase().starts_with(&format!("{}/", daily.to_lowercase()))
            || clean.eq_ignore_ascii_case(quick)
            || clean.to_lowercase().starts_with(&format!("{}/", quick.to_lowercase()))
            || clean.eq_ignore_ascii_case("quick")
            || clean.to_lowercase().starts_with("quick/")
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
            if let Err(e) = self.vault.delete(&id) {
                self.set_status(format!("Error deleting: {e}"));
            } else {
                self.editor_buffer.clear();
                self.editor_dirty = false;
                self.link_index = LinkIndex::build(&self.vault);
                self.set_status("Note deleted");
                let next_id = self.vault.notes().next().map(|n| n.id.clone());
                if let Some(id) = next_id {
                    self.open_note(id);
                }
            }
        }
    }

    pub fn open_daily_today(&mut self) {
        let today = Local::now().date_naive();
        self.selected_daily_date = today;
        match self.daily_config.open_or_create(&mut self.vault, today) {
            Ok(id) => {
                self.link_index = LinkIndex::build(&self.vault);
                self.open_note(id);
                self.top_mode = TopBarMode::Notes;
                self.set_status(format!("Opened daily note for {}", today));
            }
            Err(e) => self.set_status(format!("Failed to open daily note: {e}")),
        }
    }

    pub fn open_daily_date(&mut self, date: NaiveDate) {
        self.selected_daily_date = date;
        match self.daily_config.open_or_create(&mut self.vault, date) {
            Ok(id) => {
                self.link_index = LinkIndex::build(&self.vault);
                self.open_note(id);
                self.top_mode = TopBarMode::Notes;
                self.set_status(format!("Opened daily note for {}", date));
            }
            Err(e) => self.set_status(format!("Failed to open daily note: {e}")),
        }
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

    pub fn create_event(&mut self) {
        let draft = self.new_event_draft.clone();
        match self.event_store.create(&mut self.vault, &draft) {
            Ok(id) => {
                // Integrate directly into the daily note for that event's date
                let time_str = match (draft.start, draft.end) {
                    (Some(s), Some(e)) => format!("{} – {}", s.format("%H:%M"), e.format("%H:%M")),
                    (Some(s), None) => s.format("%H:%M").to_string(),
                    (None, _) => "All day".to_string(),
                };
                let event_bullet = format!("- [ ] **{}** [[{}]]\n", time_str, id.name());
                if let Ok(daily_id) = self.daily_config.open_or_create(&mut self.vault, draft.date) {
                    if let Some(daily_note) = self.vault.get(&daily_id) {
                        if !daily_note.content.contains(&id.name()) {
                            let _ = self.vault.append(&daily_id, &event_bullet);
                        }
                    }
                }

                self.link_index = LinkIndex::build(&self.vault);
                self.show_new_event_dialog = false;
                self.set_status("Event created and integrated into daily note!");
                self.open_note(id);
                self.top_mode = TopBarMode::Notes;
                self.sidebar_section = SidebarSection::Notes;
            }
            Err(e) => self.set_status(format!("Invalid event: {e}")),
        }
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
                if !content.contains(&ev.id.name()) {
                    let bullet = format!("- [ ] **{}** [[{}]]\n", ev.time_label(), ev.id.name());
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
}
