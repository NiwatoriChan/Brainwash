//! Extensible plugin architecture for Brainwash.
//!
//! Every feature view in Brainwash is implemented as an independent plugin implementing
//! the [`MyBrainPlugin`] trait. Plugins can be dynamically enabled and disabled at runtime.

pub mod backlinks;
pub mod canvas;
pub mod daily;
pub mod emoji;
pub mod events;
pub mod explorer;
pub mod graph;
pub mod markdown;
pub mod quick_note;

use eframe::egui;
use mybrain_core::events_bus::{AppEvent, EventBus};
use mybrain_core::index::LinkIndex;
use mybrain_core::note::NoteId;
use mybrain_core::settings::AppSettings;
use mybrain_core::vault::Vault;

use crate::theme::ThemeKind;

/// Placement slot on the vertical icon ribbon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonSlot {
    Top,
    Bottom,
}

/// Primary presentation type for a plugin's UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginViewType {
    /// Renders in the central workspace area (e.g. Markdown Editor, Graph, Daily, Canvas).
    MainWorkspace,
    /// Renders inside the collapsible left sidebar panel next to the ribbon (e.g. File Explorer).
    SidePanel,
    /// Renders inside the collapsible right panel (e.g. Connections & Backlinks).
    RightPanel,
    /// Renders as a floating modal dialog overlay (e.g. Emoji Picker).
    ModalOverlay,
    /// Headless or background service.
    Background,
}

/// Ribbon button configuration provided by a plugin.
#[derive(Debug, Clone)]
pub struct RibbonButton {
    pub icon: &'static str,
    pub tooltip: &'static str,
    pub slot: RibbonSlot,
    pub view_type: PluginViewType,
    pub priority: u8,
}

/// Command Palette action contributed by a plugin.
#[derive(Debug, Clone)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
    pub shortcut: Option<String>,
    pub plugin_id: &'static str,
}

/// Execution context passed to plugins during lifecycle, event handling, and rendering.
pub struct PluginContext<'a> {
    pub vault: &'a mut Vault,
    pub index: &'a mut LinkIndex,
    pub settings: &'a mut AppSettings,
    pub theme_kind: &'a mut ThemeKind,
    pub event_bus: &'a mut EventBus,
    pub active_note: Option<&'a NoteId>,
}

impl<'a> PluginContext<'a> {
    /// Emit an application event to the decoupled message bus.
    pub fn emit(&mut self, event: AppEvent) {
        self.event_bus.emit(event);
    }

    /// Display a transient status message in the application status bar.
    pub fn toast(&mut self, message: impl Into<String>) {
        self.event_bus.emit(AppEvent::StatusMessage(message.into()));
    }

    /// Request to open a note in the active editor.
    pub fn open_note(&mut self, note_id: NoteId) {
        self.event_bus.emit(AppEvent::RequestOpenNote(note_id));
    }

    /// Request to switch the active main workspace view.
    pub fn activate_plugin(&mut self, plugin_id: &str) {
        self.event_bus.emit(AppEvent::ActivatePlugin(plugin_id.to_string()));
    }

    /// Request to toggle a left side panel plugin view.
    pub fn toggle_side_panel(&mut self, plugin_id: &str) {
        self.event_bus.emit(AppEvent::ToggleSidePanel(plugin_id.to_string()));
    }

    /// Request to toggle the right panel plugin view.
    pub fn toggle_right_panel(&mut self, plugin_id: &str) {
        self.event_bus.emit(AppEvent::ToggleRightPanel(plugin_id.to_string()));
    }
}

/// Common trait implemented by every Brainwash feature plugin.
pub trait MyBrainPlugin: Send + Sync {
    /// Unique identifier for the plugin (e.g. "core.explorer", "core.markdown").
    fn id(&self) -> &'static str;

    /// Human-readable display name.
    fn name(&self) -> &'static str;

    /// Brief explanation of the plugin's functionality.
    fn description(&self) -> &'static str;

    /// Primary view type of this plugin.
    fn view_type(&self) -> PluginViewType {
        self.ribbon_button()
            .map(|b| b.view_type)
            .unwrap_or(PluginViewType::Background)
    }

    /// Icon button definition for the left ribbon bar, if applicable.
    fn ribbon_button(&self) -> Option<RibbonButton> {
        None
    }

    /// Called immediately when the plugin is enabled at startup or dynamically at runtime.
    fn on_enable(&mut self, _ctx: &mut PluginContext) {}

    /// Called immediately when the plugin is dynamically disabled at runtime.
    /// Use this hook to flush any dirty buffers or unsaved state to disk.
    fn on_disable(&mut self, _ctx: &mut PluginContext) {}

    /// List of commands contributed to the global Command Palette.
    fn commands(&self) -> Vec<PluginCommand> {
        Vec::new()
    }

    /// Execute a command contributed by this plugin.
    fn execute_command(&mut self, _command_id: &str, _ctx: &mut PluginContext) {}

    /// Handle events dispatched via the central [`EventBus`].
    fn on_event(&mut self, _event: &AppEvent, _ctx: &mut PluginContext) {}

    /// Render the main workspace view when this plugin is the active main plugin.
    fn render_main_view(&mut self, _ui: &mut egui::Ui, _ctx: &mut PluginContext) {}

    /// Render the side panel drawer when this plugin is the active side plugin.
    fn render_side_panel(&mut self, _ui: &mut egui::Ui, _ctx: &mut PluginContext) {}

    /// Render the right panel when this plugin is the active right plugin.
    fn render_right_panel(&mut self, ui: &mut egui::Ui, ctx: &mut PluginContext) {
        self.render_side_panel(ui, ctx);
    }

    /// Render modal or overlay dialogs when applicable.
    fn render_modal(&mut self, _ctx_egui: &egui::Context, _ctx: &mut PluginContext) {}

    /// Render custom settings rows in the Settings modal.
    fn render_settings(&mut self, _ui: &mut egui::Ui, _ctx: &mut PluginContext) {}
}

/// Registration record for an individual plugin.
pub struct PluginEntry {
    pub plugin: Box<dyn MyBrainPlugin>,
    pub enabled: bool,
}

/// Registry storing and managing all active and inactive plugins.
pub struct PluginRegistry {
    entries: Vec<PluginEntry>,
}

#[derive(Clone, Debug)]
pub struct PluginInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub view_type: PluginViewType,
    pub enabled: bool,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a plugin instance with its initial enabled state.
    pub fn register<P: MyBrainPlugin + 'static>(&mut self, plugin: P, initial_enabled: bool) {
        self.entries.push(PluginEntry {
            plugin: Box::new(plugin),
            enabled: initial_enabled,
        });
    }

    /// Check if a plugin is currently enabled.
    pub fn is_enabled(&self, id: &str) -> bool {
        self.entries
            .iter()
            .find(|e| e.plugin.id() == id)
            .map(|e| e.enabled)
            .unwrap_or(false)
    }

    /// Dynamically enable a plugin and trigger its `on_enable` lifecycle hook.
    pub fn enable_plugin(&mut self, id: &str, ctx: &mut PluginContext) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.plugin.id() == id) {
            if !entry.enabled {
                entry.enabled = true;
                ctx.settings.set_plugin_enabled(id, true);
                entry.plugin.on_enable(ctx);
            }
        }
    }

    /// Dynamically disable a plugin, flush its unsaved state via `on_disable`, and mark it inactive.
    pub fn disable_plugin(&mut self, id: &str, ctx: &mut PluginContext) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.plugin.id() == id) {
            if entry.enabled {
                entry.plugin.on_disable(ctx);
                entry.enabled = false;
                ctx.settings.set_plugin_enabled(id, false);
            }
        }
    }

    /// Returns all ribbon buttons for currently enabled plugins, sorted by priority.
    pub fn enabled_ribbon_buttons(&self) -> Vec<(&'static str, RibbonButton)> {
        let mut buttons: Vec<(&'static str, RibbonButton)> = self
            .entries
            .iter()
            .filter(|e| e.enabled)
            .filter_map(|e| e.plugin.ribbon_button().map(|btn| (e.plugin.id(), btn)))
            .collect();
        buttons.sort_by_key(|(_, b)| b.priority);
        buttons
    }

    /// Returns all commands from currently enabled plugins.
    pub fn enabled_commands(&self) -> Vec<PluginCommand> {
        let mut cmds = Vec::new();
        for entry in &self.entries {
            if entry.enabled {
                cmds.extend(entry.plugin.commands());
            }
        }
        cmds
    }

    /// Execute a command on the target enabled plugin.
    pub fn execute_command(&mut self, plugin_id: &str, command_id: &str, ctx: &mut PluginContext) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.plugin.id() == plugin_id && e.enabled) {
            entry.plugin.execute_command(command_id, ctx);
        }
    }

    /// Dispatch an event to all enabled plugins.
    pub fn dispatch_event(&mut self, event: &AppEvent, ctx: &mut PluginContext) {
        for entry in &mut self.entries {
            if entry.enabled {
                entry.plugin.on_event(event, ctx);
            }
        }
    }

    /// Get a mutable reference to a plugin's boxed trait object, regardless of enabled state.
    pub fn get_plugin_mut(&mut self, id: &str) -> Option<&mut Box<dyn MyBrainPlugin>> {
        self.entries
            .iter_mut()
            .find(|e| e.plugin.id() == id)
            .map(|e| &mut e.plugin)
    }

    /// Returns metadata summaries for all registered plugins for display in Settings.
    pub fn plugins_info(&self) -> Vec<PluginInfo> {
        self.entries
            .iter()
            .map(|e| PluginInfo {
                id: e.plugin.id(),
                name: e.plugin.name(),
                description: e.plugin.description(),
                view_type: e.plugin.view_type(),
                enabled: e.enabled,
            })
            .collect()
    }

    /// Render modal overlays for all enabled plugins.
    pub fn render_modals(&mut self, ctx_egui: &egui::Context, ctx: &mut PluginContext) {
        for entry in &mut self.entries {
            if entry.enabled {
                entry.plugin.render_modal(ctx_egui, ctx);
            }
        }
    }
}
