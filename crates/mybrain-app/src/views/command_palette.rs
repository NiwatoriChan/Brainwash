//! Command Palette / Fast Switcher modal (Ctrl+P).

use egui::{vec2, Align2, Color32, CornerRadius, Frame, Id, Margin, RichText, ScrollArea, Stroke};
use mybrain_core::NoteId;

use crate::state::{AppState, TopBarMode, ViewMode};
use crate::theme::{self, Tokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaletteCommand {
    NewNote,
    DailyToday,
    RenameActiveNote,
    ToggleViewMode,
    ToggleBacklinks,
    OpenGraph,
    OpenCalendar,
    OpenSettings,
}

impl PaletteCommand {
    fn all() -> &'static [(Self, &'static str, &'static str)] {
        &[
            (Self::NewNote, "⚡ New Note…", "Ctrl+N"),
            (Self::DailyToday, "📅 Open Today's Daily Note", "Daily"),
            (Self::RenameActiveNote, "✏️ Rename / Move Active Note…", "F2"),
            (Self::ToggleViewMode, "👁 Toggle Editor / Split / Preview", "Ctrl+E"),
            (Self::ToggleBacklinks, "🔗 Toggle Connections (Backlinks) Panel", "Links"),
            (Self::OpenGraph, "🕸 Open Knowledge Graph", "Ctrl+3"),
            (Self::OpenCalendar, "🗓 Open Calendar View", "Ctrl+2"),
            (Self::OpenSettings, "⚙ Open Preferences & Settings", "Ctrl+,"),
        ]
    }
}

#[derive(Clone, Debug)]
enum PaletteItem {
    Note(NoteId),
    Command(PaletteCommand, &'static str, &'static str),
}

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_command_palette {
        return;
    }

    let t = Tokens::get(ctx);
    let mut item_to_trigger: Option<PaletteItem> = None;
    let mut note_to_create: Option<String> = None;
    let mut close = false;

    let raw_query = state.command_palette_query.trim();
    let is_command_mode = raw_query.starts_with('>');
    let cmd_filter = raw_query.trim_start_matches('>').trim().to_lowercase();

    let mut items: Vec<PaletteItem> = Vec::new();
    if is_command_mode {
        for (cmd, label, hint) in PaletteCommand::all() {
            if cmd_filter.is_empty() || label.to_lowercase().contains(&cmd_filter) {
                items.push(PaletteItem::Command(*cmd, label, hint));
            }
        }
    } else {
        let note_hits: Vec<NoteId> = if raw_query.is_empty() {
            state.vault.notes().take(15).map(|n| n.id.clone()).collect()
        } else {
            state
                .vault
                .search(raw_query)
                .into_iter()
                .take(18)
                .map(|h| h.id)
                .collect()
        };
        for id in note_hits {
            items.push(PaletteItem::Note(id));
        }
        if !raw_query.is_empty() {
            for (cmd, label, hint) in PaletteCommand::all() {
                if label.to_lowercase().contains(&cmd_filter) {
                    items.push(PaletteItem::Command(*cmd, label, hint));
                }
            }
        }
    }

    let hits_len = items.len();
    if state.command_palette_selected >= hits_len && hits_len > 0 {
        state.command_palette_selected = hits_len - 1;
    }

    // Modal with dimmed backdrop, centered near top
    let modal = egui::Modal::new(Id::unique("command_palette_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(
            theme::modal_frame(&t)
                .inner_margin(Margin::same(14))
                .corner_radius(CornerRadius::same(t.radius_lg as u8)),
        )
        .area(
            egui::Area::new(Id::unique("command_palette_area"))
                .anchor(Align2::CENTER_TOP, vec2(0.0, 90.0))
                .order(egui::Order::Foreground)
                .interactable(true),
        );

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(540.0);

        // Header / Search input
        Frame::NONE
            .fill(t.field)
            .stroke(Stroke::new(1.0, t.field_border))
            .corner_radius(CornerRadius::same(t.radius_sm as u8))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon_search(ui, 14.0, t.text_dim);
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut state.command_palette_query)
                            .hint_text("Search notes, or type '>' for commands…")
                            .frame(Frame::NONE)
                            .desired_width(ui.available_width() - 28.0),
                    );
                    edit.request_focus();

                    if edit.changed() {
                        state.command_palette_selected = 0;
                    }

                    if !state.command_palette_query.is_empty()
                        && theme::icon_close_button(ui, &t).clicked()
                    {
                        state.command_palette_query.clear();
                        state.command_palette_selected = 0;
                    }
                });
            });

        // Keyboard navigation (Up / Down / Enter)
        let (down, up, enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Enter),
            )
        });

        if down && hits_len > 0 {
            state.command_palette_selected = (state.command_palette_selected + 1).min(hits_len - 1);
        }
        if up && hits_len > 0 {
            state.command_palette_selected = state.command_palette_selected.saturating_sub(1);
        }
        if enter {
            if hits_len > 0 && state.command_palette_selected < hits_len {
                item_to_trigger = Some(items[state.command_palette_selected].clone());
                close = true;
            } else if !is_command_mode && !state.command_palette_query.trim().is_empty() {
                note_to_create = Some(state.command_palette_query.trim().to_string());
                close = true;
            }
        }

        ui.add_space(8.0);

        // Results list
        ScrollArea::vertical()
            .id_salt("cmd_palette_scroll")
            .max_height(340.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;

                if items.is_empty() {
                    ui.add_space(14.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No matching notes or commands.").color(t.text_dim).size(13.0));
                        if !is_command_mode && !state.command_palette_query.trim().is_empty() {
                            ui.add_space(8.0);
                            let query = state.command_palette_query.trim().to_string();
                            if ui.button(RichText::new(format!("+ Create \"{query}\"")).color(t.accent_text).strong()).clicked() {
                                note_to_create = Some(query);
                                close = true;
                            }
                        }
                    });
                    ui.add_space(14.0);
                } else {
                    for (idx, item) in items.iter().enumerate() {
                        let is_active = idx == state.command_palette_selected;
                        match item {
                            PaletteItem::Note(id) => {
                                let custom_color = state.get_note_color(id);
                                let custom_emoji = state.get_note_emoji(id);
                                let is_daily = state.is_daily_note(id);
                                let is_event = state.is_event_note(id);

                                let dot = custom_color.unwrap_or_else(|| {
                                    if is_event { t.warning }
                                    else if is_daily { t.accent }
                                    else { theme::note_kind_color(id.as_str(), &t) }
                                });

                                let display_name = if let Some(e) = custom_emoji {
                                    if id.name().starts_with(&e) {
                                        id.name().to_string()
                                    } else {
                                        format!("{e} {}", id.name())
                                    }
                                } else if is_daily {
                                    format!("📅 {}", id.name())
                                } else if is_event {
                                    format!("🔔 {}", id.name())
                                } else {
                                    format!("📝 {}", id.name())
                                };

                                let folder_trail = id.folder().map(|f| {
                                    let folder_emoji = state.get_folder_emoji(f).unwrap_or_else(|| "📁".to_string());
                                    format!("{folder_emoji} {f}")
                                });

                                let row_resp = theme::list_row(
                                    ui,
                                    &t,
                                    is_active,
                                    Some(dot),
                                    &display_name,
                                    folder_trail.as_deref(),
                                );

                                if row_resp.clicked() {
                                    item_to_trigger = Some(item.clone());
                                    close = true;
                                } else if row_resp.hovered() {
                                    state.command_palette_selected = idx;
                                }
                            }
                            PaletteItem::Command(_, label, shortcut) => {
                                let row_resp = theme::list_row(
                                    ui,
                                    &t,
                                    is_active,
                                    Some(t.accent),
                                    label,
                                    Some(shortcut),
                                );
                                if row_resp.clicked() {
                                    item_to_trigger = Some(item.clone());
                                    close = true;
                                } else if row_resp.hovered() {
                                    state.command_palette_selected = idx;
                                }
                            }
                        }
                    }
                }
            });

        ui.add_space(8.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(6.0);

        // Footer shortcuts guide
        ui.horizontal(|ui| {
            let hint = |key: &str, desc: &str| {
                RichText::new(format!("[{key}] {desc}")).size(10.5).color(t.text_faint)
            };
            ui.label(hint("↑/↓", "navigate"));
            ui.label(hint("Enter", "select"));
            ui.label(hint(">", "commands"));
            ui.label(hint("Esc", "close"));
        });
    });

    if modal_resp.should_close() || close {
        state.show_command_palette = false;
        state.command_palette_query.clear();
        state.command_palette_selected = 0;
    }

    if let Some(item) = item_to_trigger {
        match item {
            PaletteItem::Note(id) => state.open_note(id),
            PaletteItem::Command(cmd, _, _) => match cmd {
                PaletteCommand::NewNote => state.show_new_note_dialog = true,
                PaletteCommand::DailyToday => state.open_daily_today(),
                PaletteCommand::RenameActiveNote => {
                    if let Some(active) = state.active_note.clone() {
                        state.open_rename_note_dialog(active);
                    }
                }
                PaletteCommand::ToggleViewMode => {
                    state.view_mode = match state.view_mode {
                        ViewMode::EditOnly => ViewMode::Split,
                        ViewMode::Split => ViewMode::PreviewOnly,
                        ViewMode::PreviewOnly => ViewMode::EditOnly,
                    };
                }
                PaletteCommand::ToggleBacklinks => {
                    state.show_backlinks = !state.show_backlinks;
                }
                PaletteCommand::OpenGraph => {
                    state.top_mode = TopBarMode::Graph;
                }
                PaletteCommand::OpenCalendar => {
                    state.top_mode = TopBarMode::Calendar;
                }
                PaletteCommand::OpenSettings => {
                    state.show_settings_dialog = true;
                }
            },
        }
    }
    if let Some(target) = note_to_create {
        state.open_or_create(&target);
    }
}
