//! Application state and actions for MyBrain.

use std::path::PathBuf;
use std::time::Instant;

use chrono::{Days, Local, NaiveDate};
use mybrain_core::{
    DailyNotes, EventDraft, EventStore, LinkIndex, NoteId, QuickNotes, Vault,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    EditOnly,
    Split,
    PreviewOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Notes,
    Daily,
    Events,
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
    pub main_tab: MainTab,
    pub search_query: String,
    pub show_backlinks: bool,

    // Modals / Overlays
    pub show_command_palette: bool,
    pub command_palette_query: String,

    pub show_quick_capture: bool,
    pub quick_capture_buffer: String,

    pub show_new_note_dialog: bool,
    pub new_note_name: String,

    pub show_new_event_dialog: bool,
    pub new_event_draft: EventDraft,

    // Daily notes navigation date
    pub selected_daily_date: NaiveDate,

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
            let welcome_id = NoteId::new("Welcome to MyBrain")?;
            let welcome_content = r#"# Welcome to MyBrain 🧠

MyBrain is a local-first, distraction-free **knowledge base** powered by Rust and egui.

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
            main_tab: MainTab::Notes,
            search_query: String::new(),
            show_backlinks: true,
            show_command_palette: false,
            command_palette_query: String::new(),
            show_quick_capture: false,
            quick_capture_buffer: String::new(),
            show_new_note_dialog: false,
            new_note_name: String::new(),
            show_new_event_dialog: false,
            new_event_draft: EventDraft::new(today),
            selected_daily_date: today,
            status: Some(("Welcome to MyBrain".to_string(), Instant::now())),
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
            self.main_tab = MainTab::Notes;
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
                self.main_tab = MainTab::Daily;
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
                self.main_tab = MainTab::Daily;
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
                self.main_tab = MainTab::Events;
            }
            Err(e) => self.set_status(format!("Invalid event: {e}")),
        }
    }
}
