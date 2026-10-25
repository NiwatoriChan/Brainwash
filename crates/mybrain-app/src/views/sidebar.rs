//! Sidebar explorer: regrouped navigation for Notes, Daily notes, and Calendar Events.

use std::collections::BTreeMap;

use chrono::Local;
use egui::{CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::{events::EventDraft, Event, NoteId};

use crate::state::{AppState, SidebarSection, TopBarMode};
use crate::theme::{self, Tokens};

#[derive(Clone, Debug)]
enum FolderAction {
    NewNote(Option<String>),
    NewEvent(Option<String>),
    NewCanvas(Option<String>),
    NewSubfolder(Option<String>),
}

#[derive(Default)]
struct FolderTree {
    subfolders: BTreeMap<String, FolderTree>,
    notes: Vec<NoteId>,
    canvases: Vec<String>,
}

impl FolderTree {
    fn is_empty(&self) -> bool {
        self.subfolders.is_empty() && self.notes.is_empty() && self.canvases.is_empty()
    }

    fn contains_active(&self, active_note: Option<&NoteId>, active_canvas: Option<&str>) -> bool {
        if let Some(an) = active_note {
            if self.notes.contains(an) {
                return true;
            }
        }
        if let Some(ac) = active_canvas {
            if self.canvases.iter().any(|c| c == ac) {
                return true;
            }
        }
        self.subfolders
            .values()
            .any(|sub| sub.contains_active(active_note, active_canvas))
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let t = Tokens::get(ui.ctx());
    ui.spacing_mut().item_spacing.y = 8.0;

    // Header: Sidebar section switcher + collapse toggle button
    ui.horizontal(|ui| {
        let section = match state.sidebar_section {
            SidebarSection::Events => SidebarSection::Daily,
            s => s,
        };
        if let Some(new_sec) = theme::segmented(
            ui,
            &t,
            section,
            &[
                (SidebarSection::Notes, "Notes"),
                (SidebarSection::Daily, "Daily"),
                (SidebarSection::Quick, "Quick"),
            ],
        ) {
            state.sidebar_section = new_sec;
            state.top_mode = TopBarMode::Notes;
            if new_sec == SidebarSection::Daily {
                state.open_daily_today();
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::icon_sidebar_button(ui, &t, true)
                .on_hover_text("Collapse sidebar (Ctrl+B)")
                .clicked()
            {
                state.show_sidebar = false;
            }
        });
    });

    ui.add_space(2.0);

    match state.sidebar_section {
        SidebarSection::Notes => {
            render_notes_section(ui, state, &t);
        }
        SidebarSection::Daily | SidebarSection::Events => {
            render_daily_section(ui, state, &t);
        }
        SidebarSection::Quick => {
            render_quick_section(ui, state, &t);
        }
    }
}

/// Helper to render an individual note item with event/daily indicators.
fn render_note_item(
    ui: &mut Ui,
    state: &AppState,
    t: &Tokens,
    id: &NoteId,
    show_folder_trail: bool,
) -> egui::Response {
    let selected = state.active_note.as_ref() == Some(id) && state.top_mode != TopBarMode::Canvas;
    let is_event = state
        .vault
        .get(id)
        .map(|n| Event::from_note(n).is_some())
        .unwrap_or(false);
    let is_daily = state.daily_config.date_of(id).is_some();

    let (dot, trail) = if is_event {
        let date_text = state
            .vault
            .get(id)
            .and_then(|n| Event::from_note(n))
            .map(|ev| ev.date.format("%b %-d").to_string());
        (Some(t.warning), date_text.or(Some("Event".to_string())))
    } else if is_daily {
        (Some(t.accent), Some("Daily".to_string()))
    } else {
        (
            Some(theme::note_kind_color(id.as_str(), t)),
            if show_folder_trail {
                id.folder().map(|s| s.to_string())
            } else {
                None
            },
        )
    };

    let resp = theme::list_row(ui, t, selected, dot, id.name(), trail.as_deref());
    if resp.drag_started() {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            crate::state::DraggedNote {
                id: id.as_str().to_string(),
                name: id.name().to_string(),
                is_canvas: false,
            },
        );
    }
    resp
}

/// Helper to render a canvas item.
fn render_canvas_item(
    ui: &mut Ui,
    state: &AppState,
    t: &Tokens,
    canvas_path: &str,
    show_folder_trail: bool,
) -> egui::Response {
    let selected = state.active_canvas_path.as_deref() == Some(canvas_path)
        && state.top_mode == TopBarMode::Canvas;
    let display_name = canvas_path
        .rsplit('/')
        .next()
        .unwrap_or(canvas_path)
        .trim_end_matches(".canvas");
    let folder_trail = if show_folder_trail {
        canvas_path.rfind('/').map(|i| &canvas_path[..i])
    } else {
        None
    };
    let dot = Some(t.accent_text);
    let trail = folder_trail.unwrap_or("Canvas");
    let resp = theme::list_row(ui, t, selected, dot, display_name, Some(trail));
    if resp.drag_started() {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            crate::state::DraggedNote {
                id: canvas_path.to_string(),
                name: display_name.to_string(),
                is_canvas: true,
            },
        );
    }
    resp
}

/// Recursively render a folder node.
fn render_folder_node(
    ui: &mut Ui,
    name: &str,
    full_path: &str,
    node: &FolderTree,
    state: &AppState,
    t: &Tokens,
    to_open_note: &mut Option<NoteId>,
    to_open_canvas: &mut Option<String>,
    folder_action: &mut Option<FolderAction>,
) {
    let id = ui.make_persistent_id(("folder_tree", full_path));
    let has_active =
        node.contains_active(state.active_note.as_ref(), state.active_canvas_path.as_deref());
    let mut collapsing =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
    if has_active {
        collapsing.set_open(true);
    }

    collapsing
        .show_header(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).strong().size(12.5).color(t.text_dim));
                let count =
                    node.notes.len() + node.canvases.len() + node.subfolders.len();
                ui.label(RichText::new(format!("{count}")).size(10.5).color(t.text_faint));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_reserved = state.is_reserved_special_folder(full_path);
                    ui.menu_button("+", |ui| {
                        if !is_reserved {
                            if ui.button("New Note").clicked() {
                                *folder_action =
                                    Some(FolderAction::NewNote(Some(full_path.to_string())));
                                ui.close();
                            }
                        }
                        if ui.button("New Event").clicked() {
                            *folder_action =
                                Some(FolderAction::NewEvent(Some(full_path.to_string())));
                            ui.close();
                        }
                        if !is_reserved {
                            if ui.button("New Canvas").clicked() {
                                *folder_action =
                                    Some(FolderAction::NewCanvas(Some(full_path.to_string())));
                                ui.close();
                            }
                            if ui.button("New Subfolder").clicked() {
                                *folder_action =
                                    Some(FolderAction::NewSubfolder(Some(full_path.to_string())));
                                ui.close();
                            }
                        }
                    });
                });
            });
        })
        .body(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;

            // 1. Subfolders
            for (sub_name, sub_node) in &node.subfolders {
                let sub_path = format!("{full_path}/{sub_name}");
                render_folder_node(
                    ui,
                    sub_name,
                    &sub_path,
                    sub_node,
                    state,
                    t,
                    to_open_note,
                    to_open_canvas,
                    folder_action,
                );
            }

            // 2. Canvases
            for canvas_path in &node.canvases {
                let resp = render_canvas_item(ui, state, t, canvas_path, false);
                if resp.clicked() {
                    *to_open_canvas = Some(canvas_path.clone());
                }
            }

            // 3. Notes (including events and daily notes in this folder)
            for note_id in &node.notes {
                let resp = render_note_item(ui, state, t, note_id, false);
                if resp.clicked() {
                    *to_open_note = Some(note_id.clone());
                }
            }

            if node.is_empty() {
                let is_reserved = state.is_reserved_special_folder(full_path);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("(empty)").color(t.text_faint).size(11.0).italics());
                    if !is_reserved {
                        if theme::ghost_button(
                            ui,
                            t,
                            RichText::new("+ note").color(t.accent_text).size(11.0),
                        )
                        .clicked()
                        {
                            *folder_action =
                                Some(FolderAction::NewNote(Some(full_path.to_string())));
                        }
                    }
                });
            }
        });
}

/// Notes explorer section: filter box, recursive folder tree, canvases, and root notes.
fn render_notes_section(ui: &mut Ui, state: &mut AppState, t: &Tokens) {
    let mut folder_action: Option<FolderAction> = None;

    // Quick action bar: section label + "+" dropdown menu
    ui.horizontal(|ui| {
        theme::section_label(ui, t, "Explorer");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button("+", |ui| {
                if ui.button("New Note (Ctrl+N)").clicked() {
                    folder_action = Some(FolderAction::NewNote(None));
                    ui.close();
                }
                if ui.button("New Event (Ctrl+Shift+E)").clicked() {
                    folder_action = Some(FolderAction::NewEvent(None));
                    ui.close();
                }
                if ui.button("New Canvas").clicked() {
                    folder_action = Some(FolderAction::NewCanvas(None));
                    ui.close();
                }
                if ui.button("New Folder").clicked() {
                    folder_action = Some(FolderAction::NewSubfolder(None));
                    ui.close();
                }
            });
        });
    });

    // Filter box
    Frame::NONE
        .fill(t.field)
        .corner_radius(CornerRadius::same(t.radius_sm as u8))
        .stroke(Stroke::new(1.0, t.field_border))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::icon_search(ui, 12.0, t.text_faint);
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Filter vault…")
                        .frame(Frame::NONE)
                        .desired_width(ui.available_width() - 26.0),
                );
                if !state.search_query.is_empty() && theme::icon_close_button(ui, t).clicked() {
                    state.search_query.clear();
                }
            });
        });

    ui.add_space(2.0);

    let mut to_open_note: Option<NoteId> = None;
    let mut to_open_canvas: Option<String> = None;

    ScrollArea::vertical()
        .id_salt("sidebar_notes_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;

            if !state.search_query.is_empty() {
                let query = state.search_query.to_lowercase();

                // Search canvases
                let canvas_files = state.vault.list_canvas_files();
                for c_path in canvas_files {
                    if c_path.to_lowercase().contains(&query) {
                        let resp = render_canvas_item(ui, state, t, &c_path, true);
                        if resp.clicked() {
                            to_open_canvas = Some(c_path);
                        }
                    }
                }

                // Search notes
                let hits: Vec<NoteId> = state
                    .vault
                    .search(&state.search_query)
                    .into_iter()
                    .map(|hit| hit.id)
                    .collect();

                if hits.is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("  No matching notes or canvases")
                            .color(t.text_faint)
                            .size(12.0),
                    );
                }
                for id in hits {
                    let resp = render_note_item(ui, state, t, &id, true);
                    if resp.clicked() {
                        to_open_note = Some(id);
                    }
                }
                return;
            }

            // Build hierarchical folder tree
            let mut root = FolderTree::default();

            // 1. Insert known folders (so empty folders appear in the tree)
            for folder in state.vault.all_folders() {
                let segments: Vec<&str> =
                    folder.split('/').filter(|s| !s.is_empty()).collect();
                let mut curr = &mut root;
                for seg in segments {
                    curr = curr.subfolders.entry(seg.to_string()).or_default();
                }
            }

            // 2. Insert canvases into the tree
            for canvas_path in state.vault.list_canvas_files() {
                if let Some(slash_idx) = canvas_path.rfind('/') {
                    let folder_part = &canvas_path[..slash_idx];
                    let segments: Vec<&str> =
                        folder_part.split('/').filter(|s| !s.is_empty()).collect();
                    let mut curr = &mut root;
                    for seg in segments {
                        curr = curr.subfolders.entry(seg.to_string()).or_default();
                    }
                    curr.canvases.push(canvas_path);
                } else {
                    root.canvases.push(canvas_path);
                }
            }

            // 3. Insert notes into the tree
            for note in state.vault.notes() {
                if let Some(folder_part) = note.id.folder() {
                    let segments: Vec<&str> =
                        folder_part.split('/').filter(|s| !s.is_empty()).collect();
                    let mut curr = &mut root;
                    for seg in segments {
                        curr = curr.subfolders.entry(seg.to_string()).or_default();
                    }
                    curr.notes.push(note.id.clone());
                } else {
                    root.notes.push(note.id.clone());
                }
            }

            // Render subfolders recursively
            for (sub_name, sub_node) in &root.subfolders {
                render_folder_node(
                    ui,
                    sub_name,
                    sub_name,
                    sub_node,
                    state,
                    t,
                    &mut to_open_note,
                    &mut to_open_canvas,
                    &mut folder_action,
                );
            }

            if !root.subfolders.is_empty() && (!root.canvases.is_empty() || !root.notes.is_empty()) {
                ui.add_space(4.0);
            }

            // Root canvases
            for canvas_path in &root.canvases {
                let resp = render_canvas_item(ui, state, t, canvas_path, false);
                if resp.clicked() {
                    to_open_canvas = Some(canvas_path.clone());
                }
            }

            // Root notes
            for note_id in &root.notes {
                let resp = render_note_item(ui, state, t, note_id, false);
                if resp.clicked() {
                    to_open_note = Some(note_id.clone());
                }
            }

            if root.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new("  Vault is empty").color(t.text_faint).size(12.0));
            }
        });

    // Handle opening selections
    if let Some(id) = to_open_note {
        state.open_note(id);
    }
    if let Some(canvas_path) = to_open_canvas {
        state.open_canvas(&canvas_path);
    }

    // Handle triggered folder actions
    if let Some(act) = folder_action {
        match act {
            FolderAction::NewNote(folder) => {
                state.new_note_folder = folder;
                state.new_note_name.clear();
                state.show_new_note_dialog = true;
            }
            FolderAction::NewEvent(folder) => {
                state.new_event_folder = folder;
                state.new_event_draft = EventDraft::new(Local::now().date_naive());
                state.show_new_event_dialog = true;
            }
            FolderAction::NewCanvas(folder) => {
                state.new_canvas_folder = folder;
                state.new_canvas_name.clear();
                state.show_new_canvas_dialog = true;
            }
            FolderAction::NewSubfolder(parent) => {
                state.new_folder_parent = parent;
                state.new_folder_name.clear();
                state.show_new_folder_dialog = true;
            }
        }
    }
}

/// Daily Notes section in sidebar: date switcher, scheduled events, and timeline entries.
fn render_daily_section(ui: &mut Ui, state: &mut AppState, t: &Tokens) {
    ui.horizontal(|ui| {
        theme::section_label(ui, t, "Daily & Events");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::ghost_button(ui, t, RichText::new("+ Event").color(t.success).size(11.5)).clicked() {
                state.new_event_draft = EventDraft::new(state.selected_daily_date);
                state.show_new_event_dialog = true;
            }
            if theme::ghost_button(ui, t, RichText::new("Today").color(t.accent_text).size(11.5)).clicked() {
                state.open_daily_today();
            }
        });
    });

    // Date Switcher Card
    Frame::NONE
        .fill(t.field)
        .stroke(Stroke::new(1.0, t.field_border))
        .corner_radius(CornerRadius::same(t.radius_sm as u8))
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if theme::ghost_button(ui, t, RichText::new("←").color(t.text_dim)).clicked() {
                    state.shift_daily_date(-1);
                }
                let date_str = state.selected_daily_date.format("%a, %b %-d").to_string();
                ui.label(RichText::new(date_str).strong().size(12.5).color(t.accent_text));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::ghost_button(ui, t, RichText::new("→").color(t.text_dim)).clicked() {
                        state.shift_daily_date(1);
                    }
                });
            });
        });

    ui.add_space(4.0);

    // Events on Selected Date
    let day_events = state.event_store.on(&state.vault, state.selected_daily_date);
    let is_today = state.selected_daily_date == Local::now().date_naive();
    let day_title = if is_today {
        "Today's Events".to_string()
    } else {
        format!("Events ({})", state.selected_daily_date.format("%b %-d"))
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(day_title).color(t.text_dim).size(11.5).strong());
        if !day_events.is_empty() {
            theme::pill(ui, &format!("{}", day_events.len()), t.warning, t.accent_soft);
        }
    });

    let mut event_to_open = None;

    if day_events.is_empty() {
        ui.label(RichText::new("  No events scheduled").color(t.text_faint).size(11.0));
    } else {
        for ev in day_events {
            let is_active = state.active_note.as_ref() == Some(&ev.id);
            let time_label = ev.time_label();
            let resp = theme::list_row(ui, t, is_active, Some(t.warning), &ev.title, Some(&time_label));
            if resp.drag_started() {
                egui::DragAndDrop::set_payload(
                    ui.ctx(),
                    crate::state::DraggedNote {
                        id: ev.id.as_str().to_string(),
                        name: ev.id.name().to_string(),
                        is_canvas: false,
                    },
                );
            }
            if resp.clicked() {
                event_to_open = Some(ev.id.clone());
            }
        }
    }

    if let Some(id) = event_to_open {
        state.open_note(id);
    }

    // Upcoming Events (next 14 days)
    let tomorrow = state.selected_daily_date.succ_opt().unwrap_or(state.selected_daily_date);
    let next_two_weeks = state.selected_daily_date.checked_add_days(chrono::Days::new(14)).unwrap_or(state.selected_daily_date);
    let upcoming = state.event_store.between(&state.vault, tomorrow, next_two_weeks);
    if !upcoming.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new("Upcoming Events").color(t.text_faint).size(11.0));
        let mut upcoming_to_open = None;
        for ev in upcoming {
            let is_active = state.active_note.as_ref() == Some(&ev.id);
            let date_str = ev.date.format("%b %-d").to_string();
            let resp = theme::list_row(ui, t, is_active, Some(t.warning), &ev.title, Some(&date_str));
            if resp.drag_started() {
                egui::DragAndDrop::set_payload(
                    ui.ctx(),
                    crate::state::DraggedNote {
                        id: ev.id.as_str().to_string(),
                        name: ev.id.name().to_string(),
                        is_canvas: false,
                    },
                );
            }
            if resp.clicked() {
                upcoming_to_open = Some(ev.id.clone());
            }
        }
        if let Some(id) = upcoming_to_open {
            state.open_note(id);
        }
    }

    ui.add_space(4.0);
    ui.label(RichText::new("Daily Notes Timeline").color(t.text_faint).size(11.0));

    let existing = state.daily_config.existing_dates(&state.vault);
    let mut to_open = None;

    ScrollArea::vertical()
        .id_salt("sidebar_daily_scroll")
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;

            if existing.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("  No daily notes yet").color(t.text_faint).size(12.0));
            } else {
                let mut dates: Vec<_> = existing.into_iter().collect();
                dates.sort_by(|a, b| b.cmp(a));

                let today = Local::now().date_naive();
                for date in dates {
                    let id = state.daily_config.id_for(date);
                    let is_active = state.active_note.as_ref() == Some(&id);
                    let is_today = date == today;

                    let trail = if is_today { Some("Today") } else { None };
                    let title = date.format("%Y-%m-%d (%a)").to_string();
                    let resp = theme::list_row(ui, t, is_active, Some(t.accent), &title, trail);
                    if resp.drag_started() {
                        egui::DragAndDrop::set_payload(
                            ui.ctx(),
                            crate::state::DraggedNote {
                                id: id.as_str().to_string(),
                                name: id.name().to_string(),
                                is_canvas: false,
                            },
                        );
                    }
                    if resp.clicked() {
                        to_open = Some(date);
                    }
                }
            }
        });

    if let Some(date) = to_open {
        state.open_daily_date(date);
    }
}

/// Google Keep / Quick Notes section in sidebar: list of quick notes with color dots and filter.
fn render_quick_section(ui: &mut Ui, state: &mut AppState, t: &Tokens) {
    ui.horizontal(|ui| {
        theme::section_label(ui, t, "Quick Notes");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::ghost_button(ui, t, RichText::new("+ Note").color(t.accent_text).size(11.5)).clicked() {
                state.quick_composer_expanded = true;
            }
        });
    });

    // Filter box
    Frame::NONE
        .fill(t.field)
        .corner_radius(CornerRadius::same(t.radius_sm as u8))
        .stroke(Stroke::new(1.0, t.field_border))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::icon_search(ui, 12.0, t.text_faint);
                ui.add(
                    egui::TextEdit::singleline(&mut state.quick_search_query)
                        .hint_text("Search Quick notes…")
                        .frame(Frame::NONE)
                        .desired_width(ui.available_width() - 26.0),
                );
                if !state.quick_search_query.is_empty() && theme::icon_close_button(ui, t).clicked() {
                    state.quick_search_query.clear();
                }
            });
        });

    ui.add_space(2.0);

    let all_notes = state.quick_config.all(&state.vault);
    let mut to_edit: Option<NoteId> = None;

    ScrollArea::vertical()
        .id_salt("sidebar_quick_scroll")
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;

            let query = state.quick_search_query.trim().to_lowercase();
            let filtered: Vec<&mybrain_core::Note> = all_notes
                .into_iter()
                .filter(|n| {
                    if query.is_empty() {
                        true
                    } else {
                        n.title().to_lowercase().contains(&query)
                            || n.body().to_lowercase().contains(&query)
                    }
                })
                .collect();

            if filtered.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("  No quick notes found").color(t.text_faint).size(12.0));
            } else {
                for note in filtered {
                    let fm = note.frontmatter();
                    let is_pinned = fm.get("pinned") == Some("true");
                    let color_name = fm.get("color");
                    let dot_color = if let Some(c) = color_name {
                        crate::views::quick_capture::QUICK_COLORS
                            .iter()
                            .find(|def| def.name.eq_ignore_ascii_case(c))
                            .map(|def| def.dot_color)
                            .unwrap_or(t.warning)
                    } else {
                        t.warning
                    };

                    let title = fm
                        .get("title")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| note.title().to_string());

                    let trail = if is_pinned { Some("Pinned") } else { None };
                    let selected = state.quick_edit_note.as_ref() == Some(&note.id);
                    let resp = theme::list_row(ui, t, selected, Some(dot_color), &title, trail);
                    if resp.drag_started() {
                        egui::DragAndDrop::set_payload(
                            ui.ctx(),
                            crate::state::DraggedNote {
                                id: note.id.as_str().to_string(),
                                name: note.id.name().to_string(),
                                is_canvas: false,
                            },
                        );
                    }
                    if resp.clicked() {
                        to_edit = Some(note.id.clone());
                    }
                }
            }
        });

    if let Some(id) = to_edit {
        state.open_quick_edit(id);
    }
}

