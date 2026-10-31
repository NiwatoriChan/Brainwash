//! Command Palette and fast switcher modal (Ctrl+P).
//! Dynamically queries PluginRegistry so commands from disabled plugins never appear.

use egui::{vec2, Align2, Color32, CornerRadius, Frame, Id, Margin, RichText, ScrollArea, Stroke};
use mybrain_core::index::LinkIndex;
use mybrain_core::NoteId;

use crate::plugins::{PluginCommand, PluginContext, PluginRegistry};
use crate::theme::{self, Tokens};

pub struct CommandPaletteState {
    pub is_open: bool,
    pub query: String,
    pub selected: usize,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            selected: 0,
        }
    }
}

enum PaletteEntry {
    Note(NoteId),
    PluginCmd(PluginCommand),
    CoreAction(&'static str, &'static str, &'static str), // id, label, shortcut
}

pub fn render_command_palette(
    ctx_egui: &egui::Context,
    palette_state: &mut CommandPaletteState,
    registry: &mut PluginRegistry,
    ctx: &mut PluginContext,
    open_settings: &mut bool,
    open_vault_switcher: &mut bool,
) {
    if !palette_state.is_open {
        return;
    }

    let t = Tokens::get(ctx_egui);
    let mut close = false;
    let mut item_to_trigger: Option<PaletteEntry> = None;
    let mut note_to_create: Option<String> = None;

    let raw_query = palette_state.query.trim().to_string();
    let is_command_mode = raw_query.starts_with('>');
    let cmd_filter = raw_query.trim_start_matches('>').trim().to_lowercase();

    let enabled_cmds = registry.enabled_commands();
    let mut entries: Vec<PaletteEntry> = Vec::new();

    // Core actions
    let core_actions = [
        ("core.settings", "⚙ Open Preferences & Settings", "Ctrl+,"),
        ("core.vault", "🗄️ Switch Vault…", "Ctrl+O"),
    ];

    if is_command_mode {
        for cmd in enabled_cmds {
            if cmd_filter.is_empty() || cmd.title.to_lowercase().contains(&cmd_filter) {
                entries.push(PaletteEntry::PluginCmd(cmd));
            }
        }
        for (id, label, shortcut) in core_actions {
            if cmd_filter.is_empty() || label.to_lowercase().contains(&cmd_filter) {
                entries.push(PaletteEntry::CoreAction(id, label, shortcut));
            }
        }
    } else {
        // Search notes
        let note_hits: Vec<NoteId> = if raw_query.is_empty() {
            ctx.vault.notes().take(15).map(|n| n.id.clone()).collect()
        } else {
            ctx.vault
                .search(&raw_query)
                .into_iter()
                .take(15)
                .map(|h| h.id)
                .collect()
        };

        for id in note_hits {
            entries.push(PaletteEntry::Note(id));
        }

        if !raw_query.is_empty() {
            for cmd in enabled_cmds {
                if cmd.title.to_lowercase().contains(&cmd_filter) {
                    entries.push(PaletteEntry::PluginCmd(cmd));
                }
            }
            for (id, label, shortcut) in core_actions {
                if label.to_lowercase().contains(&cmd_filter) {
                    entries.push(PaletteEntry::CoreAction(id, label, shortcut));
                }
            }
        }
    }

    let hits_len = entries.len();
    if palette_state.selected >= hits_len && hits_len > 0 {
        palette_state.selected = hits_len - 1;
    }

    let modal = egui::Modal::new(Id::unique("command_palette_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(
            theme::modal_frame(&t)
                .inner_margin(Margin::same(14))
                .corner_radius(CornerRadius::same(t.radius_lg as u8)),
        )
        .area(
            egui::Area::new(Id::unique("command_palette_area"))
                .anchor(Align2::CENTER_TOP, vec2(0.0, 90.0)),
        );

    let modal_resp = modal.show(ctx_egui, |ui| {
        ui.set_width(560.0);

        // Search Input Bar
        Frame::NONE
            .fill(t.field)
            .stroke(Stroke::new(1.0, t.accent_border))
            .corner_radius(CornerRadius::same(t.radius as u8))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon_search(ui, 15.0, t.accent_text);
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut palette_state.query)
                            .hint_text("Search notes or type > for plugin commands…")
                            .font(egui::TextStyle::Body)
                            .frame(Frame::NONE)
                            .desired_width(ui.available_width() - 30.0),
                    );
                    edit.request_focus();

                    if !palette_state.query.is_empty() && theme::icon_close_button(ui, &t).clicked() {
                        palette_state.query.clear();
                    }
                });
            });

        ui.add_space(8.0);

        // Keyboard navigation (Arrow keys + Enter)
        if ctx_egui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if hits_len > 0 {
                palette_state.selected = (palette_state.selected + 1) % hits_len;
            }
        }
        if ctx_egui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if hits_len > 0 {
                palette_state.selected = if palette_state.selected == 0 {
                    hits_len - 1
                } else {
                    palette_state.selected - 1
                };
            }
        }
        if ctx_egui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(entry) = entries.get(palette_state.selected) {
                item_to_trigger = Some(match entry {
                    PaletteEntry::Note(id) => PaletteEntry::Note(id.clone()),
                    PaletteEntry::PluginCmd(cmd) => PaletteEntry::PluginCmd(cmd.clone()),
                    PaletteEntry::CoreAction(id, l, s) => PaletteEntry::CoreAction(id, l, s),
                });
                close = true;
            } else if !raw_query.is_empty() && !is_command_mode {
                note_to_create = Some(raw_query.to_string());
                close = true;
            }
        }

        // Results Scroll Area
        ScrollArea::vertical()
            .max_height(340.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if entries.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        if is_command_mode {
                            ui.label(RichText::new("No matching commands").color(t.text_faint).size(13.0));
                        } else if !raw_query.is_empty() {
                            ui.label(RichText::new(format!("No note named \"{raw_query}\"")).color(t.text_faint).size(13.0));
                            ui.add_space(6.0);
                            let create_btn = egui::Button::new(
                                RichText::new(format!("+ Create note \"{raw_query}\""))
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(t.accent)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8));
                            if ui.add(create_btn).clicked() {
                                note_to_create = Some(raw_query.to_string());
                                close = true;
                            }
                        } else {
                            ui.label(RichText::new("Type to search notes or > for commands").color(t.text_faint).size(13.0));
                        }
                        ui.add_space(20.0);
                    });
                } else {
                    for (idx, entry) in entries.iter().enumerate() {
                        let is_sel = idx == palette_state.selected;
                        let row_resp = match entry {
                            PaletteEntry::Note(id) => {
                                let dot = theme::note_kind_color(id.as_str(), &t);
                                theme::list_row(ui, &t, is_sel, Some(dot), id.name(), id.folder())
                            }
                            PaletteEntry::PluginCmd(cmd) => {
                                let shortcut = cmd.shortcut.as_deref().unwrap_or("");
                                theme::list_row(ui, &t, is_sel, Some(t.accent_text), &cmd.title, Some(shortcut))
                            }
                            PaletteEntry::CoreAction(_, label, shortcut) => {
                                theme::list_row(ui, &t, is_sel, Some(t.success), label, Some(shortcut))
                            }
                        };

                        if row_resp.clicked() {
                            item_to_trigger = Some(match entry {
                                PaletteEntry::Note(id) => PaletteEntry::Note(id.clone()),
                                PaletteEntry::PluginCmd(cmd) => PaletteEntry::PluginCmd(cmd.clone()),
                                PaletteEntry::CoreAction(id, l, s) => PaletteEntry::CoreAction(id, l, s),
                            });
                            close = true;
                        }
                    }
                }
            });

        // Footer hints
        ui.add_space(6.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let hint = |s| RichText::new(s).color(t.text_faint).size(11.0);
            ui.label(hint("↑↓ to navigate · Enter to select · Esc to dismiss · > for commands"));
        });
    });

    if let Some(entry) = item_to_trigger {
        match entry {
            PaletteEntry::Note(id) => {
                ctx.open_note(id);
                ctx.activate_plugin("core.markdown");
            }
            PaletteEntry::PluginCmd(cmd) => {
                registry.execute_command(cmd.plugin_id, &cmd.id, ctx);
            }
            PaletteEntry::CoreAction(id, _, _) => match id {
                "core.settings" => *open_settings = true,
                "core.vault" => *open_vault_switcher = true,
                _ => {}
            },
        }
    }

    if let Some(name) = note_to_create {
        if let Ok(id) = NoteId::new(&name) {
            let initial = format!("# {}\n\n", id.name());
            let _ = ctx.vault.create(id.clone(), &initial);
            *ctx.index = LinkIndex::build(ctx.vault);
            ctx.open_note(id);
            ctx.activate_plugin("core.markdown");
        }
    }

    if modal_resp.should_close() || close || ctx_egui.input(|i| i.key_pressed(egui::Key::Escape)) {
        palette_state.is_open = false;
        palette_state.query.clear();
    }
}
