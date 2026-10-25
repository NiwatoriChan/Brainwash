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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DailyViewMode {
    MonthGrid,
    Timeline,
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

    pub fn create_note(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        match NoteId::new(name) {
            Ok(id) => {
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
                self.sidebar_section = SidebarSection::Daily;
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
                self.sidebar_section = SidebarSection::Daily;
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

    pub fn create_event(&mut self) {
        match self.event_store.create(&mut self.vault, &self.new_event_draft) {
            Ok(id) => {
                self.link_index = LinkIndex::build(&self.vault);
                self.show_new_event_dialog = false;
                self.set_status("Event created!");
                self.open_note(id);
                self.top_mode = TopBarMode::Notes;
                self.sidebar_section = SidebarSection::Events;
            }
            Err(e) => self.set_status(format!("Invalid event: {e}")),
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
        let rel = match parent {
            Some(p) if !p.is_empty() => format!("{p}/{clean}"),
            _ => clean.to_owned(),
        };
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
