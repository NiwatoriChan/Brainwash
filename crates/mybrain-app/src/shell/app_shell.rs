//! Main application shell coordinating vault, index, settings, event bus, and plugins.

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui;
use egui::{vec2, Align2, Color32, CornerRadius, CursorIcon, Frame, Margin, RichText, Sense, Stroke};
use mybrain_core::daily::DailyNotes;
use mybrain_core::events_bus::{AppEvent, EventBus};
use mybrain_core::index::LinkIndex;
use mybrain_core::note::NoteId;
use mybrain_core::settings::AppSettings;
use mybrain_core::vault::Vault;

use crate::plugins::explorer::DraggedNote;
use crate::plugins::{
    PluginContext, PluginRegistry, PluginViewType,
};
use crate::shell::command_palette::{self, CommandPaletteState};
use crate::shell::dialogs::{self, DialogsState};
use crate::shell::ribbon;
use crate::shell::settings_modal::{self, SettingsModalState};
use crate::shell::vault_switcher::{self, VaultSwitcherState};
use crate::theme::{self, ThemeKind, Tokens};

pub const STATUS_TTL: std::time::Duration = std::time::Duration::from_secs(4);

pub struct AppShell {
    pub vault: Vault,
    pub index: LinkIndex,
    pub settings: AppSettings,
    pub settings_path: PathBuf,
    pub theme_kind: ThemeKind,
    pub event_bus: EventBus,
    pub plugins: PluginRegistry,

    pub active_main_plugin: Option<String>,
    pub active_side_plugin: Option<String>,
    pub active_right_plugin: Option<String>,
    pub active_note: Option<NoteId>,

    pub status: Option<(String, Instant)>,

    // Core Shell Dialogs & Modals
    pub command_palette_state: CommandPaletteState,
    pub vault_switcher_state: VaultSwitcherState,
    pub settings_modal_state: SettingsModalState,
    pub show_settings_dialog: bool,
    pub dialogs_state: DialogsState,
}

impl AppShell {
    pub fn new(vault_path: PathBuf) -> Result<Self, mybrain_core::Error> {
        let vault = Vault::open(&vault_path)?;
        let index = LinkIndex::build(&vault);
        let settings_path = vault_path.join(".mybrain").join("settings.json");
        let settings = AppSettings::load_from_file(&settings_path);
        let theme_kind = ThemeKind::from_str_or_default(&settings.theme);
        let event_bus = EventBus::new();
        let mut plugins = PluginRegistry::new();

        // Register all 9 built-in plugins
        plugins.register(crate::plugins::explorer::ExplorerPlugin::new(), settings.is_plugin_enabled("core.explorer"));
        plugins.register(crate::plugins::markdown::MarkdownPlugin::new(), settings.is_plugin_enabled("core.markdown"));
        plugins.register(crate::plugins::quick_note::QuickNotePlugin::new(), settings.is_plugin_enabled("core.quick_note"));
        plugins.register(crate::plugins::daily::DailyPlugin::new(), settings.is_plugin_enabled("core.daily"));
        plugins.register(crate::plugins::canvas::CanvasPlugin::new(), settings.is_plugin_enabled("core.canvas"));
        plugins.register(crate::plugins::graph::GraphPlugin::new(), settings.is_plugin_enabled("core.graph"));
        plugins.register(crate::plugins::events::EventsPlugin::new(), settings.is_plugin_enabled("core.events"));
        plugins.register(crate::plugins::backlinks::BacklinksPlugin::new(), settings.is_plugin_enabled("core.backlinks"));
        plugins.register(crate::plugins::emoji::EmojiPlugin::new(), settings.is_plugin_enabled("core.emoji"));

        let active_main = if settings.is_plugin_enabled("core.markdown") {
            Some("core.markdown".to_string())
        } else {
            None
        };

        let active_side = if settings.is_plugin_enabled("core.explorer") {
            Some("core.explorer".to_string())
        } else {
            None
        };

        // Connections right panel is closed by default
        let active_right = None;

        Ok(Self {
            vault,
            index,
            settings,
            settings_path,
            theme_kind,
            event_bus,
            plugins,
            active_main_plugin: active_main,
            active_side_plugin: active_side,
            active_right_plugin: active_right,
            active_note: None,
            status: None,
            command_palette_state: CommandPaletteState::default(),
            vault_switcher_state: VaultSwitcherState::default(),
            settings_modal_state: SettingsModalState::default(),
            show_settings_dialog: false,
            dialogs_state: DialogsState::default(),
        })
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), Instant::now()));
    }

    pub fn active_status(&self) -> Option<&str> {
        self.status.as_ref().and_then(|(msg, at)| {
            if at.elapsed() < STATUS_TTL {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    pub fn is_reserved_special_folder(&self, folder: &str) -> bool {
        let clean = folder.trim().trim_matches('/');
        let daily = self.settings.daily_folder.trim().trim_matches('/');
        let events = self.settings.events_folder.trim().trim_matches('/');

        clean.eq_ignore_ascii_case(daily)
            || clean.to_lowercase().starts_with(&format!("{}/", daily.to_lowercase()))
            || clean.eq_ignore_ascii_case(events)
            || clean.to_lowercase().starts_with(&format!("{}/", events.to_lowercase()))
            || clean.eq_ignore_ascii_case("Quick Notes")
            || clean.to_lowercase().starts_with("quick notes/")
            || clean.eq_ignore_ascii_case("quick")
            || clean.to_lowercase().starts_with("quick/")
            || clean.eq_ignore_ascii_case("events")
            || clean.to_lowercase().starts_with("events/")
    }

    pub fn is_daily_note(&self, id: &NoteId) -> bool {
        let daily = DailyNotes {
            folder: self.settings.daily_folder.clone(),
            ..Default::default()
        };
        daily.date_of(id).is_some()
    }

    pub fn is_event_note(&self, id: &NoteId) -> bool {
        id.as_str().starts_with(&format!("{}/", self.settings.events_folder))
            || self.vault.get(id).map_or(false, |n| mybrain_core::Event::from_note(n).is_some())
    }

    pub fn is_daily_or_event_folder(&self, folder: &str) -> bool {
        let clean = folder.trim().trim_matches('/');
        let daily = self.settings.daily_folder.trim().trim_matches('/');
        let events = self.settings.events_folder.trim().trim_matches('/');

        clean.eq_ignore_ascii_case(daily)
            || clean.to_lowercase().starts_with(&format!("{}/", daily.to_lowercase()))
            || clean.eq_ignore_ascii_case(events)
            || clean.to_lowercase().starts_with(&format!("{}/", events.to_lowercase()))
    }

    pub fn get_folder_emoji(&self, folder: &str) -> Option<String> {
        let clean = folder.trim_end_matches('/');
        self.settings.folder_emojis.get(clean).cloned()
    }

    pub fn set_folder_emoji(&mut self, folder: &str, emoji: Option<&str>) {
        let clean = folder.trim_end_matches('/').to_string();
        if let Some(e) = emoji {
            self.settings.folder_emojis.insert(clean, e.to_string());
        } else {
            self.settings.folder_emojis.remove(&clean);
        }
        let _ = self.settings.save_to_file(&self.settings_path);
    }

    pub fn get_folder_color(&self, folder: &str) -> Option<Color32> {
        let clean = folder.trim_end_matches('/');
        self.settings
            .folder_colors
            .get(clean)
            .and_then(|c| theme::parse_color_str(c))
    }

    pub fn get_folder_color_name(&self, folder: &str) -> Option<String> {
        let clean = folder.trim_end_matches('/');
        self.settings.folder_colors.get(clean).cloned()
    }

    pub fn set_folder_color(&mut self, folder: &str, color: Option<&str>) {
        let clean = folder.trim_end_matches('/').to_string();
        if let Some(c) = color {
            self.settings.folder_colors.insert(clean, c.to_string());
        } else {
            self.settings.folder_colors.remove(&clean);
        }
        let _ = self.settings.save_to_file(&self.settings_path);
    }

    pub fn get_note_color(&self, id: &NoteId) -> Option<Color32> {
        let note = self.vault.get(id)?;
        let fm = note.frontmatter();
        let color_val = fm.get("color")?;
        theme::parse_color_str(color_val)
    }

    pub fn get_note_color_name(&self, id: &NoteId) -> Option<String> {
        let note = self.vault.get(id)?;
        let fm = note.frontmatter();
        fm.get("color").map(ToString::to_string)
    }

    pub fn set_note_color(&mut self, id: &NoteId, color: Option<&str>) {
        if let Some(note) = self.vault.get(id) {
            let (mut fm, body) = mybrain_core::frontmatter::Frontmatter::parse(&note.content);
            if let Some(c) = color {
                fm.set("color", c.trim());
            } else {
                fm.remove("color");
            }
            let new_content = if fm.is_empty() {
                body.trim_start_matches('\n').to_string()
            } else {
                format!("{}{}", fm.render(), body.trim_start_matches('\n'))
            };
            let _ = self.vault.save(id, &new_content);
            self.index = mybrain_core::index::LinkIndex::build(&self.vault);
            self.event_bus.emit(AppEvent::NoteModified(id.clone()));
        }
    }

    pub fn get_note_emoji(&self, id: &NoteId) -> Option<String> {
        let note = self.vault.get(id)?;
        let fm = note.frontmatter();
        if let Some(icon) = fm.get("icon") {
            return Some(icon.to_string());
        }
        theme::split_leading_emoji(id.name()).map(|(e, _)| e.to_string())
    }

    pub fn set_note_emoji(&mut self, id: &NoteId, emoji: Option<&str>) {
        if let Some(note) = self.vault.get(id) {
            let (mut fm, body) = mybrain_core::frontmatter::Frontmatter::parse(&note.content);
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
            self.index = mybrain_core::index::LinkIndex::build(&self.vault);
            self.event_bus.emit(AppEvent::NoteModified(id.clone()));
        }
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
            return Err(mybrain_core::Error::InvalidName("Cannot create folder with reserved name".into()));
        }
        self.vault.create_folder(&rel)?;
        Ok(())
    }

    pub fn switch_vault(&mut self, new_root: PathBuf) -> Result<(), mybrain_core::Error> {
        let mut new_vault = Vault::open(&new_root)?;
        if new_vault.is_empty() {
            let welcome_id = NoteId::new("Welcome to Brainwash")?;
            let _ = new_vault.create(welcome_id.clone(), "# Welcome to Brainwash\nYour fresh new modular vault.");
        }
        self.index = LinkIndex::build(&new_vault);
        self.settings.record_recent_vault(&new_root.display().to_string());
        self.vault = new_vault;
        self.settings_path = new_root.join(".mybrain").join("settings.json");
        let _ = self.settings.save_to_file(&self.settings_path);
        self.active_note = None;
        self.set_status("Switched vault");
        Ok(())
    }

    pub fn process_events(&mut self) {
        let events = self.event_bus.drain();
        for event in events {
            match &event {
                AppEvent::ActivatePlugin(id) => {
                    if self.plugins.is_enabled(id) {
                        self.active_main_plugin = Some(id.clone());
                    }
                }
                AppEvent::ToggleSidePanel(id) => {
                    if self.active_side_plugin.as_deref() == Some(id) {
                        self.active_side_plugin = None;
                    } else if self.plugins.is_enabled(id) {
                        self.active_side_plugin = Some(id.clone());
                    }
                }
                AppEvent::ToggleRightPanel(id) => {
                    if self.active_right_plugin.as_deref() == Some(id) {
                        self.active_right_plugin = None;
                    } else if self.plugins.is_enabled(id) {
                        self.active_right_plugin = Some(id.clone());
                    }
                }
                AppEvent::OpenModal(name) => {
                    if name == "settings" {
                        self.show_settings_dialog = true;
                    } else if name == "vault_switcher" {
                        self.vault_switcher_state.is_open = true;
                    } else if name == "command_palette" {
                        self.command_palette_state.is_open = true;
                    } else if name == "new_note" {
                        self.dialogs_state.show_new_note = true;
                    } else if name.starts_with("new_folder") {
                        let parent = name.strip_prefix("new_folder:").filter(|s| !s.is_empty()).map(ToString::to_string);
                        self.dialogs_state.new_folder_parent = parent;
                        self.dialogs_state.show_new_folder = true;
                    } else if name == "emoji_picker" {
                        if let Some(entry) = self.plugins.get_plugin_mut("core.emoji") {
                            entry.on_event(&event, &mut PluginContext {
                                vault: &mut self.vault,
                                index: &mut self.index,
                                settings: &mut self.settings,
                                theme_kind: &mut self.theme_kind,
                                event_bus: &mut self.event_bus,
                                active_note: self.active_note.as_ref(),
                            });
                        }
                    }
                }
                AppEvent::RequestNewNote(folder) => {
                    self.dialogs_state.new_note_folder = folder.clone();
                    self.dialogs_state.show_new_note = true;
                }
                AppEvent::StatusMessage(msg) => {
                    if let Some(emoji) = msg.strip_prefix("EMOJI_NEW_NOTE:") {
                        self.dialogs_state.new_note_emoji = Some(emoji.to_string());
                    } else if let Some(emoji) = msg.strip_prefix("EMOJI_NEW_FOLDER:") {
                        self.dialogs_state.new_folder_emoji = Some(emoji.to_string());
                    } else if let Some(target) = msg.strip_prefix("RENAME_NOTE:") {
                        if let Ok(id) = NoteId::new(target) {
                            self.dialogs_state.rename_target = Some(id.clone());
                            self.dialogs_state.rename_new_name = id.as_str().to_string();
                            self.dialogs_state.show_rename_note = true;
                        }
                    } else {
                        self.set_status(msg.clone());
                    }
                }
                AppEvent::RequestOpenNote(id) => {
                    self.active_note = Some(id.clone());
                    if self.plugins.is_enabled("core.markdown") {
                        self.active_main_plugin = Some("core.markdown".into());
                    }
                }
                AppEvent::RequestOpenCanvas(_) => {
                    if self.plugins.is_enabled("core.canvas") {
                        self.active_main_plugin = Some("core.canvas".into());
                    }
                }
                AppEvent::NoteDeleted(id) => {
                    if self.active_note.as_ref() == Some(id) {
                        self.active_note = None;
                    }
                }
                AppEvent::NoteRenamed { old, new } => {
                    if self.active_note.as_ref() == Some(old) {
                        self.active_note = Some(new.clone());
                    }
                }
                _ => {}
            }

            // Forward to all enabled plugins
            let mut ctx = PluginContext {
                vault: &mut self.vault,
                index: &mut self.index,
                settings: &mut self.settings,
                theme_kind: &mut self.theme_kind,
                event_bus: &mut self.event_bus,
                active_note: self.active_note.as_ref(),
            };
            self.plugins.dispatch_event(&event, &mut ctx);
        }
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+P: Command Palette
            if i.modifiers.command && i.key_pressed(egui::Key::P) {
                self.command_palette_state.is_open = !self.command_palette_state.is_open;
            }
            // Ctrl+Shift+B: Toggle Right Panel (Connections)
            if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::B) {
                if self.active_right_plugin.is_some() {
                    self.active_right_plugin = None;
                } else if self.plugins.is_enabled("core.backlinks") {
                    self.active_right_plugin = Some("core.backlinks".into());
                }
            }
            // Ctrl+B: Toggle Left Side Panel
            if i.modifiers.command && !i.modifiers.shift && (i.key_pressed(egui::Key::B) || i.key_pressed(egui::Key::Backslash)) {
                if self.active_side_plugin.is_some() {
                    self.active_side_plugin = None;
                } else if self.plugins.is_enabled("core.explorer") {
                    self.active_side_plugin = Some("core.explorer".into());
                }
            }
            // Ctrl+N: New Note
            if i.modifiers.command && i.key_pressed(egui::Key::N) {
                self.dialogs_state.show_new_note = true;
            }
            // Ctrl+,: Open Settings
            if i.modifiers.command && i.key_pressed(egui::Key::Comma) {
                self.show_settings_dialog = true;
            }
            // Ctrl+O: Vault Switcher
            if i.modifiers.command && i.key_pressed(egui::Key::O) {
                self.vault_switcher_state.is_open = true;
            }
            // Ctrl+1: Markdown, Ctrl+2: Daily, Ctrl+3: Graph
            if i.modifiers.command && i.key_pressed(egui::Key::Num1) {
                if self.plugins.is_enabled("core.markdown") {
                    self.active_main_plugin = Some("core.markdown".into());
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num2) {
                if self.plugins.is_enabled("core.daily") {
                    self.active_main_plugin = Some("core.daily".into());
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num3) {
                if self.plugins.is_enabled("core.graph") {
                    self.active_main_plugin = Some("core.graph".into());
                }
            }
            // Ctrl+Q: Quick Note
            if i.modifiers.command && i.key_pressed(egui::Key::Q) {
                if self.plugins.is_enabled("core.quick_note") {
                    let mut p_ctx = PluginContext {
                        vault: &mut self.vault,
                        index: &mut self.index,
                        settings: &mut self.settings,
                        theme_kind: &mut self.theme_kind,
                        event_bus: &mut self.event_bus,
                        active_note: self.active_note.as_ref(),
                    };
                    self.plugins.execute_command("core.quick_note", "quick_capture", &mut p_ctx);
                }
            }
            // Ctrl+S: Save
            if i.modifiers.command && i.key_pressed(egui::Key::S) {
                if self.plugins.is_enabled("core.markdown") {
                    let mut p_ctx = PluginContext {
                        vault: &mut self.vault,
                        index: &mut self.index,
                        settings: &mut self.settings,
                        theme_kind: &mut self.theme_kind,
                        event_bus: &mut self.event_bus,
                        active_note: self.active_note.as_ref(),
                    };
                    self.plugins.execute_command("core.markdown", "save_note", &mut p_ctx);
                }
            }
            // Ctrl+W: Close Tab
            if i.modifiers.command && i.key_pressed(egui::Key::W) {
                if self.plugins.is_enabled("core.markdown") {
                    let mut p_ctx = PluginContext {
                        vault: &mut self.vault,
                        index: &mut self.index,
                        settings: &mut self.settings,
                        theme_kind: &mut self.theme_kind,
                        event_bus: &mut self.event_bus,
                        active_note: self.active_note.as_ref(),
                    };
                    self.plugins.execute_command("core.markdown", "close_tab", &mut p_ctx);
                }
            }
            // Ctrl+Tab: Next Tab
            if i.modifiers.command && i.key_pressed(egui::Key::Tab) {
                if self.plugins.is_enabled("core.markdown") {
                    let mut p_ctx = PluginContext {
                        vault: &mut self.vault,
                        index: &mut self.index,
                        settings: &mut self.settings,
                        theme_kind: &mut self.theme_kind,
                        event_bus: &mut self.event_bus,
                        active_note: self.active_note.as_ref(),
                    };
                    self.plugins.execute_command("core.markdown", "cycle_tab", &mut p_ctx);
                }
            }
            // Ctrl+E: Toggle View Mode
            if i.modifiers.command && i.key_pressed(egui::Key::E) {
                if self.plugins.is_enabled("core.markdown") {
                    let mut p_ctx = PluginContext {
                        vault: &mut self.vault,
                        index: &mut self.index,
                        settings: &mut self.settings,
                        theme_kind: &mut self.theme_kind,
                        event_bus: &mut self.event_bus,
                        active_note: self.active_note.as_ref(),
                    };
                    self.plugins.execute_command("core.markdown", "toggle_view_mode", &mut p_ctx);
                }
            }
        });
    }

    pub fn top_bar(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        egui::Panel::top("top_bar")
            .exact_size(44.0)
            .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin { left: 10, right: 8, top: 0, bottom: 0 }))
            .show(ui, |ui| {
                let top_bar_rect = ui.max_rect();
                theme::bottom_hairline(ui, top_bar_rect, t);

                // Window drag handle
                let title_bar_response = ui.interact(
                    top_bar_rect,
                    ui.scope_id().with("fused_title_bar_drag_handle"),
                    Sense::click_and_drag(),
                );
                if title_bar_response.double_clicked() {
                    let is_max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                } else if title_bar_response.drag_started_by(egui::PointerButton::Primary) {
                    ui.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.horizontal_centered(|ui| {
                    // Brand Mark
                    let (mark, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(mark, CornerRadius::same(5), t.accent);
                    ui.painter().circle_filled(mark.center(), 3.5, t.card);
                    ui.label(RichText::new("Brainwash").strong().size(15.0).color(t.text));

                    // Vault switcher button
                    let vault_name = self.vault.root().file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Vault".to_string());
                    if theme::ghost_button(ui, t, RichText::new(format!("{vault_name} ▾")).size(12.5).color(t.text_dim))
                        .on_hover_text("Open or switch vault (Ctrl+O)")
                        .clicked()
                    {
                        self.vault_switcher_state.is_open = true;
                    }

                    ui.add_space(12.0);

                    // Search field / Command Palette trigger
                    let search_w = (ui.available_width() - 360.0).clamp(160.0, 360.0);
                    let (rect, resp) = ui.allocate_exact_size(vec2(search_w, 28.0), Sense::click());
                    let hovered = resp.hovered();
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(t.radius as u8),
                        if hovered { t.hover } else { t.field },
                        Stroke::new(1.0, t.field_border),
                        egui::StrokeKind::Inside,
                    );
                    theme::paint_search_icon(
                        ui.painter(),
                        rect.left_center() + vec2(14.0, 0.0),
                        13.0,
                        t.text_faint,
                    );
                    ui.painter().text(
                        rect.left_center() + vec2(28.0, 0.0),
                        Align2::LEFT_CENTER,
                        "Search notes or type > for commands…",
                        egui::FontId::proportional(12.0),
                        t.text_faint,
                    );
                    ui.painter().text(
                        rect.right_center() - vec2(10.0, 0.0),
                        Align2::RIGHT_CENTER,
                        "Ctrl P",
                        egui::FontId::monospace(10.5),
                        t.text_faint,
                    );
                    if resp.on_hover_cursor(CursorIcon::Text).clicked() {
                        self.command_palette_state.is_open = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Window controls on non-macOS
                        #[cfg(not(target_os = "macos"))]
                        {
                            let is_max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                            if let Some(action) = theme::window_control_buttons_rtl(ui, t, is_max) {
                                match action {
                                    theme::WindowControlAction::Minimize => {
                                        ui.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                                    }
                                    theme::WindowControlAction::MaximizeToggle => {
                                        ui.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                                    }
                                    theme::WindowControlAction::Close => {
                                        ui.send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                }
                            }
                            ui.add_space(6.0);
                            let (sep_rect, _) = ui.allocate_exact_size(vec2(1.0, 16.0), Sense::hover());
                            ui.painter().rect_filled(sep_rect, CornerRadius::ZERO, t.separator);
                            ui.add_space(6.0);
                        }

                        // Settings button
                        if theme::ghost_button(ui, t, RichText::new("⚙ Settings").color(t.text_dim))
                            .on_hover_text("Preferences & Plugin Manager (Ctrl+,)")
                            .clicked()
                        {
                            self.show_settings_dialog = true;
                        }

                        ui.add_space(4.0);

                        // Connections right panel toggle button (simple icon)
                        let is_right_open = self.active_right_plugin.is_some();
                        let (btn_rect, btn_resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
                        if ui.is_rect_visible(btn_rect) {
                            let painter = ui.painter();
                            if is_right_open {
                                painter.rect_filled(btn_rect, CornerRadius::same(t.radius_sm as u8), t.accent.gamma_multiply(0.2));
                                painter.rect_stroke(btn_rect, CornerRadius::same(t.radius_sm as u8), Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
                            } else if btn_resp.hovered() {
                                painter.rect_filled(btn_rect, CornerRadius::same(t.radius_sm as u8), t.hover);
                            }
                            let icon_color = if is_right_open {
                                t.accent_text
                            } else if btn_resp.hovered() {
                                t.text
                            } else {
                                t.text_dim
                            };
                            painter.text(
                                btn_rect.center(),
                                Align2::CENTER_CENTER,
                                "🔗",
                                egui::FontId::proportional(14.0),
                                icon_color,
                            );
                        }
                        if btn_resp
                            .on_hover_text("Toggle Connections & Backlinks (Ctrl+Shift+B)")
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            if self.active_right_plugin.is_some() {
                                self.active_right_plugin = None;
                            } else if self.plugins.is_enabled("core.backlinks") {
                                self.active_right_plugin = Some("core.backlinks".into());
                            }
                        }

                        ui.add_space(4.0);

                        // New Note button
                        if ui
                            .add(
                                egui::Button::new(RichText::new("+ Note").strong().color(Color32::WHITE))
                                    .fill(t.accent)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                            )
                            .on_hover_text("New note (Ctrl+N)")
                            .clicked()
                        {
                            self.dialogs_state.show_new_note = true;
                        }
                    });
                });
            });
    }

    pub fn status_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, t: &Tokens) {
        egui::Panel::bottom("status_bar")
            .exact_size(26.0)
            .frame(
                Frame::NONE
                    .fill(t.chrome)
                    .stroke(Stroke::new(1.0, t.separator))
                    .inner_margin(Margin::symmetric(14, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let small = |s: String, c| RichText::new(s).size(11.0).color(c);
                    ui.label(small(format!("{} notes", self.vault.len()), t.text_faint));

                    if let Some(ref note_id) = self.active_note {
                        if let Some(note) = self.vault.get(note_id) {
                            let words = note.content.split_whitespace().count();
                            let chars = note.content.chars().count();
                            ui.label(small("·".into(), t.text_faint));
                            ui.label(small(format!("{words} words"), t.text_faint));
                            ui.label(small("·".into(), t.text_faint));
                            ui.label(small(format!("{chars} characters"), t.text_faint));
                        }
                    }

                    let enabled_count = self.plugins.plugins_info().into_iter().filter(|p| p.enabled).count();
                    ui.label(small("·".into(), t.text_faint));
                    ui.label(small(format!("{enabled_count} plugins active"), t.text_faint));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some((_, at)) = &self.status {
                            let age = at.elapsed();
                            if age < STATUS_TTL {
                                let remaining = (STATUS_TTL - age).as_secs_f32();
                                let alpha = remaining.clamp(0.0, 1.0);
                                let msg = self.active_status().unwrap_or_default().to_owned();
                                ui.label(small(msg, t.accent_text.gamma_multiply(alpha)));
                                ctx.request_repaint_after(std::time::Duration::from_millis(50));
                            }
                        }
                    });
                });
            });
    }

    pub fn render_fallback_view(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        ui.centered_and_justified(|ui| {
            Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.card_border))
                .corner_radius(CornerRadius::same(t.radius as u8))
                .inner_margin(Margin::same(32))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 20,
                    spread: 0,
                    color: t.shadow,
                })
                .show(ui, |ui| {
                    ui.set_max_width(440.0);
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("No Active Workspace View").size(18.0).color(t.text).strong());
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("The previously active view is closed or disabled. Select an enabled feature below to open it, or configure plugins in Settings:")
                                .color(t.text_dim)
                                .size(13.0),
                        );

                        ui.add_space(18.0);
                        let enabled_workspaces: Vec<_> = self.plugins.plugins_info()
                            .into_iter()
                            .filter(|p| p.enabled && p.view_type == PluginViewType::MainWorkspace)
                            .collect();

                        for plugin in enabled_workspaces {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(format!("Open {}", plugin.name)).strong().color(Color32::WHITE))
                                        .fill(t.accent)
                                        .stroke(Stroke::NONE)
                                        .corner_radius(CornerRadius::same(t.radius_sm as u8))
                                        .min_size(vec2(220.0, 32.0)),
                                )
                                .clicked()
                            {
                                self.active_main_plugin = Some(plugin.id.to_string());
                            }
                            ui.add_space(6.0);
                        }

                        ui.add_space(10.0);
                        if theme::ghost_button(ui, t, RichText::new("⚙️ Open Plugin Manager").color(t.accent_text)).clicked() {
                            self.settings_modal_state.active_tab = settings_modal::SettingsTab::Plugins;
                            self.show_settings_dialog = true;
                        }
                    });
                });
        });
    }
}

impl eframe::App for AppShell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.process_events();
        self.handle_shortcuts(&ctx);

        let tokens = Tokens::get(&ctx);

        self.top_bar(ui, &tokens);
        self.status_bar(ui, &ctx, &tokens);

        // 1. Permanent Left Ribbon (always visible)
        let ribbon_action = egui::Panel::left("shell_ribbon_panel")
            .exact_size(44.0)
            .resizable(false)
            .frame(Frame::NONE.fill(tokens.dock))
            .show(ui, |ui| {
                ribbon::render_ribbon(
                    ui,
                    &self.plugins,
                    self.active_main_plugin.as_deref(),
                    self.active_side_plugin.as_deref(),
                    self.active_right_plugin.as_deref(),
                    &tokens,
                )
            })
            .inner;

        if let Some(side) = ribbon_action.toggle_side {
            if self.active_side_plugin.as_deref() == Some(&side) {
                self.active_side_plugin = None;
            } else {
                self.active_side_plugin = Some(side);
            }
        }
        if let Some(right) = ribbon_action.toggle_right {
            if self.active_right_plugin.as_deref() == Some(&right) {
                self.active_right_plugin = None;
            } else {
                self.active_right_plugin = Some(right);
            }
        }
        if let Some(main) = ribbon_action.switch_main {
            self.active_main_plugin = Some(main);
        }
        if let Some(modal) = ribbon_action.open_modal {
            self.event_bus.emit(AppEvent::OpenModal(modal));
        }
        if ribbon_action.open_command_palette {
            self.command_palette_state.is_open = true;
        }
        if ribbon_action.open_vault_switcher {
            self.vault_switcher_state.is_open = true;
        }
        if ribbon_action.open_settings {
            self.show_settings_dialog = true;
        }

        // 2. Collapsible Left Side Panel (only visible if active_side_plugin is Some and enabled)
        if let Some(ref side_id) = self.active_side_plugin.clone() {
            if self.plugins.is_enabled(side_id) {
                egui::Panel::left("shell_side_panel")
                    .default_size(260.0)
                    .min_size(200.0)
                    .resizable(true)
                    .frame(
                        Frame::NONE
                            .fill(tokens.dock)
                            .stroke(Stroke::new(1.0, tokens.separator))
                            .inner_margin(Margin::symmetric(10, 12)),
                    )
                    .show(ui, |ui| {
                        let mut p_ctx = PluginContext {
                            vault: &mut self.vault,
                            index: &mut self.index,
                            settings: &mut self.settings,
                            theme_kind: &mut self.theme_kind,
                            event_bus: &mut self.event_bus,
                            active_note: self.active_note.as_ref(),
                        };
                        if let Some(entry) = self.plugins.get_plugin_mut(side_id) {
                            entry.render_side_panel(ui, &mut p_ctx);
                        }
                    });
            }
        }

        // 3. Collapsible Right Panel (Connections & Backlinks)
        if let Some(ref right_id) = self.active_right_plugin.clone() {
            if self.plugins.is_enabled(right_id) {
                egui::Panel::right("shell_right_panel")
                    .default_size(280.0)
                    .min_size(200.0)
                    .max_size(480.0)
                    .resizable(true)
                    .frame(
                        Frame::NONE
                            .fill(tokens.dock)
                            .stroke(Stroke::new(1.0, tokens.separator))
                            .inner_margin(Margin::symmetric(10, 12)),
                    )
                    .show(ui, |ui| {
                        let mut p_ctx = PluginContext {
                            vault: &mut self.vault,
                            index: &mut self.index,
                            settings: &mut self.settings,
                            theme_kind: &mut self.theme_kind,
                            event_bus: &mut self.event_bus,
                            active_note: self.active_note.as_ref(),
                        };
                        if let Some(entry) = self.plugins.get_plugin_mut(right_id) {
                            entry.render_right_panel(ui, &mut p_ctx);
                        }
                    });
            }
        }

        // 4. Central Main Workspace
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(tokens.canvas).inner_margin(Margin::ZERO))
            .show(ui, |ui| {
                let mut rendered = false;
                if let Some(ref main_id) = self.active_main_plugin.clone() {
                    if self.plugins.is_enabled(main_id) {
                        let mut p_ctx = PluginContext {
                            vault: &mut self.vault,
                            index: &mut self.index,
                            settings: &mut self.settings,
                            theme_kind: &mut self.theme_kind,
                            event_bus: &mut self.event_bus,
                            active_note: self.active_note.as_ref(),
                        };
                        if let Some(entry) = self.plugins.get_plugin_mut(main_id) {
                            entry.render_main_view(ui, &mut p_ctx);
                            rendered = true;
                        }
                    }
                }

                if !rendered {
                    self.render_fallback_view(ui, &tokens);
                }
            });

        // 5. Modals and Dialogs
        let mut switch_vault_req = None;
        {
            let mut p_ctx = PluginContext {
                vault: &mut self.vault,
                index: &mut self.index,
                settings: &mut self.settings,
                theme_kind: &mut self.theme_kind,
                event_bus: &mut self.event_bus,
                active_note: self.active_note.as_ref(),
            };

            // Command Palette
            command_palette::render_command_palette(
                &ctx,
                &mut self.command_palette_state,
                &mut self.plugins,
                &mut p_ctx,
                &mut self.show_settings_dialog,
                &mut self.vault_switcher_state.is_open,
            );

            // Settings Modal
            settings_modal::render_settings_modal(
                &ctx,
                &mut self.settings_modal_state,
                &mut self.show_settings_dialog,
                &mut self.plugins,
                &mut self.active_main_plugin,
                &mut self.active_side_plugin,
                &mut self.active_right_plugin,
                &mut p_ctx,
                &self.settings_path,
            );

            // Vault Switcher Modal
            vault_switcher::render_vault_switcher(
                &ctx,
                &mut self.vault_switcher_state,
                &mut p_ctx,
                &mut switch_vault_req,
            );

            // Shared Dialogs (New Note, Rename, New Folder, Custom Color)
            dialogs::render_dialogs(&ctx, &mut self.dialogs_state, &mut p_ctx);

            // Plugin Modals (Emoji Picker, Quick Capture, etc.)
            self.plugins.render_modals(&ctx, &mut p_ctx);
        }

        if let Some(new_vault_path) = switch_vault_req {
            let _ = self.switch_vault(new_vault_path);
        }

        // Drag preview tooltip
        if let Some(dragged) = egui::DragAndDrop::payload::<DraggedNote>(&ctx) {
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                egui::Area::new(egui::Id::unique("drag_preview_area"))
                    .fixed_pos(pos + vec2(16.0, 16.0))
                    .order(egui::Order::Tooltip)
                    .interactable(false)
                    .show(&ctx, |ui| {
                        Frame::NONE
                            .fill(tokens.card)
                            .stroke(Stroke::new(1.0, tokens.accent))
                            .corner_radius(CornerRadius::same(tokens.radius_sm as u8))
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🔗").size(12.0));
                                    ui.label(RichText::new(format!("[[{}]]", dragged.name)).strong().size(12.0).color(tokens.text));
                                });
                            });
                    });
            }
        }
        if ctx.input(|i| i.pointer.any_released()) {
            let _ = egui::DragAndDrop::take_payload::<DraggedNote>(&ctx);
        }

        // Handle window frame and edge resizing on non-macOS
        handle_window_frame_and_resize(&ctx, &tokens);
    }
}

fn handle_window_frame_and_resize(ctx: &egui::Context, t: &Tokens) {
    #[cfg(not(target_os = "macos"))]
    {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let screen_rect = ctx.viewport_rect();

        if !is_maximized {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("custom_window_border")))
                .rect_stroke(
                    screen_rect,
                    CornerRadius::same(6),
                    Stroke::new(1.0, t.separator),
                    egui::StrokeKind::Inside,
                );

            if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                let border_thick = 6.0;
                let near_left = pos.x <= screen_rect.left() + border_thick && pos.x >= screen_rect.left() - 2.0;
                let near_right = pos.x >= screen_rect.right() - border_thick && pos.x <= screen_rect.right() + 2.0;
                let near_top = pos.y <= screen_rect.top() + border_thick && pos.y >= screen_rect.top() - 2.0;
                let near_bottom = pos.y >= screen_rect.bottom() - border_thick && pos.y <= screen_rect.bottom() + 2.0;

                let in_y = pos.y >= screen_rect.top() && pos.y <= screen_rect.bottom();
                let in_x = pos.x >= screen_rect.left() && pos.x <= screen_rect.right();

                let resize_info = if near_top && near_left {
                    Some((egui::viewport::ResizeDirection::NorthWest, CursorIcon::ResizeNorthWest))
                } else if near_top && near_right {
                    Some((egui::viewport::ResizeDirection::NorthEast, CursorIcon::ResizeNorthEast))
                } else if near_bottom && near_left {
                    Some((egui::viewport::ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest))
                } else if near_bottom && near_right {
                    Some((egui::viewport::ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast))
                } else if near_top && in_x {
                    Some((egui::viewport::ResizeDirection::North, CursorIcon::ResizeNorth))
                } else if near_bottom && in_x {
                    Some((egui::viewport::ResizeDirection::South, CursorIcon::ResizeSouth))
                } else if near_left && in_y {
                    Some((egui::viewport::ResizeDirection::West, CursorIcon::ResizeWest))
                } else if near_right && in_y {
                    Some((egui::viewport::ResizeDirection::East, CursorIcon::ResizeEast))
                } else {
                    None
                };

                if let Some((dir, cursor)) = resize_info {
                    ctx.set_cursor_icon(cursor);
                    if ctx.input(|i| i.pointer.primary_down()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_app_shell() -> (tempfile::TempDir, AppShell) {
        let tmp = tempfile::tempdir().unwrap();
        let shell = AppShell::new(tmp.path().to_path_buf()).unwrap();
        (tmp, shell)
    }

    #[test]
    fn test_is_reserved_special_folder() {
        let (_tmp, shell) = temp_app_shell();
        assert!(shell.is_reserved_special_folder("Daily"));
        assert!(shell.is_reserved_special_folder("daily"));
        assert!(shell.is_reserved_special_folder("Daily/2026"));
        assert!(shell.is_reserved_special_folder("Quick Notes"));
        assert!(shell.is_reserved_special_folder("quick notes"));
        assert!(shell.is_reserved_special_folder("Quick Notes/Archive"));
        assert!(shell.is_reserved_special_folder("quick"));
        assert!(shell.is_reserved_special_folder("quick/foo"));
        assert!(!shell.is_reserved_special_folder("Projects"));
        assert!(!shell.is_reserved_special_folder("Work/DailyTasks"));
        assert!(!shell.is_reserved_special_folder("Notes"));
    }

    #[test]
    fn test_is_daily_and_event_isolation() {
        let (_tmp, mut shell) = temp_app_shell();
        let daily_id = NoteId::new("Daily/2026-10-25").unwrap();
        let event_id = NoteId::new("Events/Meeting").unwrap();
        let normal_id = NoteId::new("Projects/Brainwash").unwrap();

        shell.vault.create(daily_id.clone(), "# 2026-10-25\n").unwrap();
        shell.vault.create(event_id.clone(), "---\ntype: event\ndate: 2026-10-25\n---\n# Meeting\n").unwrap();
        shell.vault.create(normal_id.clone(), "# Brainwash\n").unwrap();

        assert!(shell.is_daily_note(&daily_id));
        assert!(!shell.is_daily_note(&normal_id));
        assert!(!shell.is_daily_note(&event_id));

        assert!(shell.is_event_note(&event_id));
        assert!(!shell.is_event_note(&normal_id));

        assert!(shell.is_daily_or_event_folder("Daily"));
        assert!(shell.is_daily_or_event_folder("daily/2026"));
        assert!(shell.is_daily_or_event_folder("Events"));
        assert!(!shell.is_daily_or_event_folder("Projects"));
    }

    #[test]
    fn test_folder_emoji() {
        let (_tmp, mut shell) = temp_app_shell();
        assert_eq!(shell.get_folder_emoji("Projects"), None);

        shell.set_folder_emoji("Projects", Some("📁"));
        assert_eq!(shell.get_folder_emoji("Projects"), Some("📁".to_string()));
        assert_eq!(shell.get_folder_emoji("Projects/"), Some("📁".to_string()));

        shell.set_folder_emoji("Projects", None);
        assert_eq!(shell.get_folder_emoji("Projects"), None);
    }

    #[test]
    fn test_folder_color() {
        let (_tmp, mut shell) = temp_app_shell();
        assert_eq!(shell.get_folder_color("Projects"), None);

        shell.set_folder_color("Projects", Some("teal"));
        assert!(shell.get_folder_color("Projects").is_some());
        assert_eq!(shell.get_folder_color_name("Projects"), Some("teal".to_string()));
        assert_eq!(shell.get_folder_color_name("Projects/"), Some("teal".to_string()));

        shell.set_folder_color("Projects", None);
        assert_eq!(shell.get_folder_color("Projects"), None);
    }

    #[test]
    fn test_note_custom_color_and_emoji() {
        let (_tmp, mut shell) = temp_app_shell();
        let note_id = NoteId::new("Projects/Idea").unwrap();
        shell.vault.create(note_id.clone(), "# Idea\n").unwrap();

        assert_eq!(shell.get_note_color(&note_id), None);
        assert_eq!(shell.get_note_emoji(&note_id), None);

        shell.set_note_color(&note_id, Some("coral"));
        assert!(shell.get_note_color(&note_id).is_some());

        shell.set_note_emoji(&note_id, Some("🚀"));
        assert_eq!(shell.get_note_emoji(&note_id), Some("🚀".to_string()));

        shell.set_note_color(&note_id, None);
        assert_eq!(shell.get_note_color(&note_id), None);
        shell.set_note_emoji(&note_id, None);
        assert_eq!(shell.get_note_emoji(&note_id), None);
    }

    #[test]
    fn test_plugin_dynamic_enable_disable_and_fallback() {
        let (_tmp, mut shell) = temp_app_shell();
        assert!(shell.plugins.is_enabled("core.quick_note"));

        // By default, markdown, canvas, and backlinks do NOT have ribbon buttons on the left bar
        let initial_buttons = shell.plugins.enabled_ribbon_buttons();
        assert!(!initial_buttons.iter().any(|(id, _)| *id == "core.markdown" || *id == "core.canvas" || *id == "core.backlinks"));
        // But quick_note and explorer DO have ribbon buttons
        assert!(initial_buttons.iter().any(|(id, _)| *id == "core.quick_note"));
        assert!(initial_buttons.iter().any(|(id, _)| *id == "core.explorer"));

        // Disable quick_note plugin dynamically
        let mut p_ctx = PluginContext {
            vault: &mut shell.vault,
            index: &mut shell.index,
            settings: &mut shell.settings,
            theme_kind: &mut shell.theme_kind,
            event_bus: &mut shell.event_bus,
            active_note: shell.active_note.as_ref(),
        };
        shell.plugins.disable_plugin("core.quick_note", &mut p_ctx);
        assert!(!shell.plugins.is_enabled("core.quick_note"));

        // When quick_note is disabled, ribbon buttons no longer contain it
        let buttons = shell.plugins.enabled_ribbon_buttons();
        assert!(!buttons.iter().any(|(id, _)| *id == "core.quick_note"));

        // Commands no longer contain quick_note commands
        let commands = shell.plugins.enabled_commands();
        assert!(!commands.iter().any(|c| c.plugin_id == "core.quick_note"));

        // Re-enable quick_note plugin
        let mut p_ctx2 = PluginContext {
            vault: &mut shell.vault,
            index: &mut shell.index,
            settings: &mut shell.settings,
            theme_kind: &mut shell.theme_kind,
            event_bus: &mut shell.event_bus,
            active_note: shell.active_note.as_ref(),
        };
        shell.plugins.enable_plugin("core.quick_note", &mut p_ctx2);
        assert!(shell.plugins.is_enabled("core.quick_note"));
        let buttons2 = shell.plugins.enabled_ribbon_buttons();
        assert!(buttons2.iter().any(|(id, _)| *id == "core.quick_note"));
    }

    #[test]
    fn test_right_panel_toggle_and_connection() {
        let (_tmp, mut shell) = temp_app_shell();
        // Closed by default
        assert_eq!(shell.active_right_plugin, None);

        // Toggle right panel open
        shell.event_bus.emit(AppEvent::ToggleRightPanel("core.backlinks".into()));
        shell.process_events();
        assert_eq!(shell.active_right_plugin.as_deref(), Some("core.backlinks"));

        // Toggle right panel closed
        shell.event_bus.emit(AppEvent::ToggleRightPanel("core.backlinks".into()));
        shell.process_events();
        assert_eq!(shell.active_right_plugin, None);
    }
}
