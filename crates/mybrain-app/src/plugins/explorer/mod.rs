//! File Explorer side panel plugin for Brainwash.

use std::collections::{BTreeMap, HashSet};

use egui::{vec2, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::note::NoteId;

use crate::plugins::emoji::EmojiPickerTarget;
use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::{self, Tokens};

#[derive(Clone, Debug)]
pub struct DraggedNote {
    pub id: String,
    pub name: String,
    pub is_canvas: bool,
}

#[derive(Clone, Debug)]
pub enum FolderAction {
    NewNote(Option<String>),
    NewCanvas(Option<String>),
    NewSubfolder(Option<String>),
}

#[derive(Clone, Debug)]
pub enum TreeAction {
    Folder(FolderAction),
    SetNoteColor(NoteId, Option<String>),
    SetNoteEmoji(NoteId, Option<String>),
    SetNoteEmojiAndColor(NoteId, Option<String>, Option<String>),
    RenameNote(NoteId),
    DeleteNote(NoteId),
    SetFolderEmoji(String, Option<String>),
    SetFolderColor(String, Option<String>),
    SetFolderEmojiAndColor(String, Option<String>, Option<String>),
    OpenEmojiPicker(EmojiPickerTarget),
}

#[derive(Default)]
struct FolderTree {
    subfolders: BTreeMap<String, FolderTree>,
    notes: Vec<NoteId>,
    canvases: Vec<String>,
    drawings: Vec<String>,
}

impl FolderTree {
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.subfolders.is_empty() && self.notes.is_empty() && self.canvases.is_empty() && self.drawings.is_empty()
    }

    fn contains_active(&self, active_note: Option<&NoteId>) -> bool {
        if let Some(an) = active_note {
            if self.notes.contains(an) {
                return true;
            }
        }
        self.subfolders
            .values()
            .any(|sub| sub.contains_active(active_note))
    }
}

pub struct ExplorerPlugin {
    pub search_query: String,
    pub expanded_folders: HashSet<String>,
}

impl ExplorerPlugin {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            expanded_folders: HashSet::new(),
        }
    }
}

impl Default for ExplorerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for ExplorerPlugin {
    fn id(&self) -> &'static str {
        "core.explorer"
    }

    fn name(&self) -> &'static str {
        "File Explorer"
    }

    fn description(&self) -> &'static str {
        "Hierarchical file and folder tree navigation with search, icons, and organization"
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "📁",
            tooltip: "File Explorer (Ctrl+B)",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::SidePanel,
            priority: 10,
        })
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "toggle_sidebar".into(),
                title: "Toggle File Explorer Side Panel".into(),
                shortcut: Some("Ctrl+B".into()),
                plugin_id: "core.explorer",
            },
            PluginCommand {
                id: "new_note".into(),
                title: "Create New Note".into(),
                shortcut: Some("Ctrl+N".into()),
                plugin_id: "core.explorer",
            },
            PluginCommand {
                id: "new_folder".into(),
                title: "Create New Folder".into(),
                shortcut: None,
                plugin_id: "core.explorer",
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        match command_id {
            "toggle_sidebar" => {
                ctx.toggle_side_panel("core.explorer");
            }
            "new_note" => {
                ctx.emit(AppEvent::RequestNewNote(None));
            }
            "new_folder" => {
                ctx.emit(AppEvent::OpenModal("new_folder".into()));
            }
            _ => {}
        }
    }

    fn render_side_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());
        ui.spacing_mut().item_spacing.y = 8.0;

        // Header: Explorer title, new buttons, collapse button
        ui.horizontal(|ui| {
            theme::section_label(ui, &t, "Explorer");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_sidebar_button(ui, &t, true)
                    .on_hover_text("Collapse explorer panel (Ctrl+B)")
                    .clicked()
                {
                    ctx.toggle_side_panel("core.explorer");
                }

                if theme::ghost_button(ui, &t, RichText::new("+ Folder").color(t.text_dim).size(11.5))
                    .on_hover_text("Create new folder")
                    .clicked()
                {
                    ctx.emit(AppEvent::OpenModal("new_folder".into()));
                }

                if theme::ghost_button(ui, &t, RichText::new("+ Drawing").color(t.accent_text).size(11.5))
                    .on_hover_text("Open Drawing Documents")
                    .clicked()
                {
                    ctx.emit(AppEvent::RequestOpenDrawing(String::new()));
                }

                if theme::ghost_button(ui, &t, RichText::new("+ Note").color(t.accent_text).strong().size(11.5))
                    .on_hover_text("Create new note (Ctrl+N)")
                    .clicked()
                {
                    ctx.emit(AppEvent::RequestNewNote(None));
                }
            });
        });

        ui.add_space(2.0);

        // Search Bar
        Frame::NONE
            .fill(t.field)
            .stroke(Stroke::new(1.0, t.field_border))
            .corner_radius(CornerRadius::same(t.radius_sm as u8))
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon_search(ui, 13.0, t.text_faint);
                    let _edit = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Filter notes & folders…")
                            .frame(Frame::NONE)
                            .desired_width(ui.available_width() - 20.0),
                    );

                    if !self.search_query.is_empty()
                        && theme::icon_close_button(ui, &t).clicked()
                    {
                        self.search_query.clear();
                    }
                });
            });

        ui.add_space(4.0);

        let mut tree_action: Option<TreeAction> = None;
        let mut note_to_open: Option<NoteId> = None;
        let mut canvas_to_open: Option<String> = None;
        let mut drawing_to_open: Option<String> = None;

        ScrollArea::vertical()
            .id_salt("explorer_tree_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;

                let query = self.search_query.trim().to_lowercase();
                if !query.is_empty() {
                    // Search Results List
                    let results: Vec<NoteId> = ctx.vault
                        .notes()
                        .filter(|n| {
                            n.id.name().to_lowercase().contains(&query)
                                || n.id.as_str().to_lowercase().contains(&query)
                        })
                        .map(|n| n.id.clone())
                        .collect();

                    if results.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(RichText::new("No matching notes found").color(t.text_faint).size(12.0));
                        });
                    } else {
                        for id in results {
                            let selected = ctx.active_note == Some(&id);
                            let dot = theme::note_kind_color(id.as_str(), &t);
                            let resp = theme::list_row(ui, &t, selected, Some(dot), id.name(), id.folder());
                            if resp.clicked() {
                                note_to_open = Some(id);
                            }
                        }
                    }
                } else {
                    // Hierarchical Folder Tree
                    let mut root = FolderTree::default();

                    for note in ctx.vault.notes() {
                        let path = note.id.as_str();
                        let parts: Vec<&str> = path.split('/').collect();
                        if parts.len() == 1 {
                            root.notes.push(note.id.clone());
                        } else {
                            let mut current = &mut root;
                            for &part in &parts[..parts.len() - 1] {
                                current = current.subfolders.entry(part.to_string()).or_default();
                            }
                            current.notes.push(note.id.clone());
                        }
                    }

                    for canvas_path in ctx.vault.list_canvas_files() {
                        let parts: Vec<&str> = canvas_path.split('/').collect();
                        if parts.len() == 1 {
                            root.canvases.push(canvas_path);
                        } else {
                            let mut current = &mut root;
                            for &part in &parts[..parts.len() - 1] {
                                current = current.subfolders.entry(part.to_string()).or_default();
                            }
                            current.canvases.push(canvas_path);
                        }
                    }

                    for drawing_path in ctx.vault.list_drawing_files() {
                        let parts: Vec<&str> = drawing_path.split('/').collect();
                        if parts.len() == 1 {
                            root.drawings.push(drawing_path);
                        } else {
                            let mut current = &mut root;
                            for &part in &parts[..parts.len() - 1] {
                                current = current.subfolders.entry(part.to_string()).or_default();
                            }
                            current.drawings.push(drawing_path);
                        }
                    }

                    // Render root-level notes first
                    for id in &root.notes {
                        let selected = ctx.active_note == Some(id);
                        let note = ctx.vault.get(id);
                        let note_fm = note.map(|n| n.frontmatter());
                        let custom_color = note_fm.as_ref().and_then(|fm| fm.get("color")).and_then(crate::theme::parse_color_str);
                        let custom_emoji = note_fm.as_ref().and_then(|fm| fm.get("icon")).map(|s| s.to_string());

                        let dot = custom_color.unwrap_or_else(|| theme::note_kind_color(id.as_str(), &t));
                        let display_name = if let Some(ref e) = custom_emoji {
                            format!("{e} {}", id.name())
                        } else {
                            format!("📝 {}", id.name())
                        };

                        let resp = theme::list_row(ui, &t, selected, Some(dot), &display_name, None);
                        if resp.drag_started() {
                            egui::DragAndDrop::set_payload(
                                ui.ctx(),
                                DraggedNote {
                                    id: id.as_str().to_string(),
                                    name: id.name().to_string(),
                                    is_canvas: false,
                                },
                            );
                        }
                        if resp.clicked() {
                            note_to_open = Some(id.clone());
                        }

                        resp.context_menu(|ui| {
                            if ui.button("🗑 Delete Note").clicked() {
                                tree_action = Some(TreeAction::DeleteNote(id.clone()));
                                ui.close();
                            }
                            if ui.button("✏️ Rename Note…").clicked() {
                                tree_action = Some(TreeAction::RenameNote(id.clone()));
                                ui.close();
                            }
                            if ui.button("🏷️ Set Icon / Color…").clicked() {
                                tree_action = Some(TreeAction::OpenEmojiPicker(EmojiPickerTarget::Note(id.clone())));
                                ui.close();
                            }
                        });
                    }

                    // Render root canvases
                    for canvas in &root.canvases {
                        let label = canvas.trim_end_matches(".canvas");
                        let resp = theme::list_row(ui, &t, false, Some(t.accent_text), &format!("🎨 {label}"), None);
                        if resp.clicked() {
                            canvas_to_open = Some(canvas.clone());
                        }
                    }

                    // Render root drawings
                    for drawing in &root.drawings {
                        let label = drawing.split('/').last().unwrap_or(drawing).trim_end_matches(".drawing").trim_end_matches(".noteful").trim_end_matches(".json");
                        let resp = theme::list_row(ui, &t, false, Some(egui::Color32::from_rgb(251, 146, 60)), &format!("🎨 {label}"), None);
                        if resp.clicked() {
                            drawing_to_open = Some(drawing.clone());
                        }
                    }

                    // Render subfolders recursively
                    for (name, sub) in &root.subfolders {
                        render_folder_recursive(
                            ui,
                            ctx,
                            &t,
                            name,
                            name,
                            sub,
                            0,
                            &mut self.expanded_folders,
                            &mut tree_action,
                            &mut note_to_open,
                            &mut canvas_to_open,
                            &mut drawing_to_open,
                        );
                    }
                }
            });

        // Handle navigation and actions
        if let Some(id) = note_to_open {
            ctx.open_note(id);
            ctx.activate_plugin("core.markdown");
        }

        if let Some(canvas) = canvas_to_open {
            ctx.emit(AppEvent::RequestOpenCanvas(canvas));
        }

        if let Some(drawing) = drawing_to_open {
            ctx.emit(AppEvent::RequestOpenDrawing(drawing));
        }

        if let Some(action) = tree_action {
            match action {
                TreeAction::DeleteNote(id) => {
                    let _ = ctx.vault.delete(&id);
                    ctx.emit(AppEvent::NoteDeleted(id));
                    ctx.toast("Note deleted");
                }
                TreeAction::RenameNote(id) => {
                    ctx.emit(AppEvent::StatusMessage(format!("RENAME_NOTE:{}", id.as_str())));
                }
                TreeAction::OpenEmojiPicker(target) => match target {
                    EmojiPickerTarget::Note(id) => ctx.emit(AppEvent::OpenModal(format!("emoji_note:{}", id.as_str()))),
                    EmojiPickerTarget::Folder(folder) => ctx.emit(AppEvent::OpenModal(format!("emoji_folder:{}", folder))),
                    _ => ctx.emit(AppEvent::OpenModal("emoji_picker".into())),
                },
                TreeAction::Folder(FolderAction::NewNote(folder)) => {
                    ctx.emit(AppEvent::RequestNewNote(folder));
                }
                TreeAction::Folder(FolderAction::NewSubfolder(parent)) => {
                    ctx.emit(AppEvent::OpenModal(format!("new_folder:{}", parent.unwrap_or_default())));
                }
                TreeAction::Folder(FolderAction::NewCanvas(folder)) => {
                    ctx.emit(AppEvent::OpenModal(format!("new_canvas:{}", folder.unwrap_or_default())));
                }
                _ => {}
            }
        }
    }
}

fn render_folder_recursive(
    ui: &mut Ui,
    ctx: &PluginContext,
    t: &Tokens,
    folder_name: &str,
    full_path: &str,
    tree: &FolderTree,
    depth: usize,
    expanded_folders: &mut HashSet<String>,
    tree_action: &mut Option<TreeAction>,
    note_to_open: &mut Option<NoteId>,
    canvas_to_open: &mut Option<String>,
    drawing_to_open: &mut Option<String>,
) {
    let clean_path = full_path.trim_end_matches('/').to_string();
    let is_expanded = expanded_folders.contains(&clean_path) || tree.contains_active(ctx.active_note);

    let folder_emoji = ctx.settings.folder_emojis.get(&clean_path).cloned().unwrap_or_else(|| "📁".to_string());
    let folder_color = ctx.settings.folder_colors.get(&clean_path).and_then(|c| crate::theme::parse_color_str(c));

    let indent = (depth as f32) * 14.0;
    ui.horizontal(|ui| {
        if indent > 0.0 {
            ui.add_space(indent);
        }

        let arrow = if is_expanded { "▼" } else { "▶" };
        let arrow_resp = ui.add(
            egui::Button::new(RichText::new(arrow).size(10.0).color(t.text_faint))
                .fill(egui::Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .min_size(vec2(14.0, 20.0)),
        );

        if arrow_resp.clicked() {
            if is_expanded {
                expanded_folders.remove(&clean_path);
            } else {
                expanded_folders.insert(clean_path.clone());
            }
        }

        let dot_color = folder_color.unwrap_or(t.text_dim);
        let folder_label = format!("{folder_emoji} {folder_name}");
        let resp = theme::list_row(ui, t, false, Some(dot_color), &folder_label, None);

        if resp.clicked() {
            if is_expanded {
                expanded_folders.remove(&clean_path);
            } else {
                expanded_folders.insert(clean_path.clone());
            }
        }

        resp.context_menu(|ui| {
            if ui.button("+ New Note in Folder").clicked() {
                *tree_action = Some(TreeAction::Folder(FolderAction::NewNote(Some(clean_path.clone()))));
                ui.close();
            }
            if ui.button("+ New Subfolder").clicked() {
                *tree_action = Some(TreeAction::Folder(FolderAction::NewSubfolder(Some(clean_path.clone()))));
                ui.close();
            }
            if ui.button("+ New Canvas").clicked() {
                *tree_action = Some(TreeAction::Folder(FolderAction::NewCanvas(Some(clean_path.clone()))));
                ui.close();
            }
            ui.separator();
            if ui.button("🏷️ Set Folder Icon…").clicked() {
                *tree_action = Some(TreeAction::OpenEmojiPicker(EmojiPickerTarget::Folder(clean_path.clone())));
                ui.close();
            }
        });
    });

    if is_expanded {
        for id in &tree.notes {
            let note_indent = indent + 18.0;
            ui.horizontal(|ui| {
                ui.add_space(note_indent);
                let selected = ctx.active_note == Some(id);
                let note = ctx.vault.get(id);
                let note_fm = note.map(|n| n.frontmatter());
                let custom_color = note_fm.as_ref().and_then(|fm| fm.get("color")).and_then(crate::theme::parse_color_str);
                let custom_emoji = note_fm.as_ref().and_then(|fm| fm.get("icon")).map(|s| s.to_string());

                let dot = custom_color.unwrap_or_else(|| theme::note_kind_color(id.as_str(), t));
                let display_name = if let Some(ref e) = custom_emoji {
                    format!("{e} {}", id.name())
                } else {
                    format!("📝 {}", id.name())
                };

                let resp = theme::list_row(ui, t, selected, Some(dot), &display_name, None);
                if resp.drag_started() {
                    egui::DragAndDrop::set_payload(
                        ui.ctx(),
                        DraggedNote {
                            id: id.as_str().to_string(),
                            name: id.name().to_string(),
                            is_canvas: false,
                        },
                    );
                }
                if resp.clicked() {
                    *note_to_open = Some(id.clone());
                }

                resp.context_menu(|ui| {
                    if ui.button("🗑 Delete Note").clicked() {
                        *tree_action = Some(TreeAction::DeleteNote(id.clone()));
                        ui.close();
                    }
                    if ui.button("✏️ Rename Note…").clicked() {
                        *tree_action = Some(TreeAction::RenameNote(id.clone()));
                        ui.close();
                    }
                    if ui.button("🏷️ Set Icon / Color…").clicked() {
                        *tree_action = Some(TreeAction::OpenEmojiPicker(EmojiPickerTarget::Note(id.clone())));
                        ui.close();
                    }
                });
            });
        }

        for canvas in &tree.canvases {
            let canvas_indent = indent + 18.0;
            ui.horizontal(|ui| {
                ui.add_space(canvas_indent);
                let label = canvas.trim_end_matches(".canvas");
                let resp = theme::list_row(ui, t, false, Some(t.accent_text), &format!("🎨 {label}"), None);
                if resp.clicked() {
                    *canvas_to_open = Some(canvas.clone());
                }
            });
        }

        for drawing in &tree.drawings {
            let drawing_indent = indent + 18.0;
            ui.horizontal(|ui| {
                ui.add_space(drawing_indent);
                let label = drawing.split('/').last().unwrap_or(drawing).trim_end_matches(".drawing").trim_end_matches(".noteful").trim_end_matches(".json");
                let resp = theme::list_row(ui, t, false, Some(egui::Color32::from_rgb(251, 146, 60)), &format!("🎨 {label}"), None);
                if resp.clicked() {
                    *drawing_to_open = Some(drawing.clone());
                }
            });
        }

        for (sub_name, sub_tree) in &tree.subfolders {
            let next_full = format!("{clean_path}/{sub_name}");
            render_folder_recursive(
                ui,
                ctx,
                t,
                sub_name,
                &next_full,
                sub_tree,
                depth + 1,
                expanded_folders,
                tree_action,
                note_to_open,
                canvas_to_open,
                drawing_to_open,
            );
        }
    }
}
