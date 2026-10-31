//! Obsidian-compatible 2D Visual Canvas editor plugin for Brainwash.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Id, Margin,
    PointerButton, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2,
};
use mybrain_core::canvas::{CanvasFile, CanvasNode};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::note::NoteId;

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton,
};
use crate::theme::{self, Tokens};

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasState {
    pub pan: Vec2,
    pub zoom: f32,
    pub selected_node: Option<String>,
    pub dragging_node: Option<String>,
    pub connecting_from: Option<String>,
    pub snap_to_grid: bool,
    pub editing_text_id: Option<String>,
    pub note_picker_open: bool,
    pub note_picker_query: String,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected_node: None,
            dragging_node: None,
            connecting_from: None,
            snap_to_grid: true,
            editing_text_id: None,
            note_picker_open: false,
            note_picker_query: String::new(),
        }
    }
}

pub struct CanvasPlugin {
    pub active_path: Option<String>,
    pub doc: Option<CanvasFile>,
    pub dirty: bool,
    pub state: CanvasState,
}

impl CanvasPlugin {
    pub fn new() -> Self {
        Self {
            active_path: None,
            doc: None,
            dirty: false,
            state: CanvasState::default(),
        }
    }

    pub fn open_canvas(&mut self, rel_path: &str, ctx: &mut PluginContext) {
        match ctx.vault.load_canvas(rel_path) {
            Ok(doc) => {
                self.active_path = Some(rel_path.to_owned());
                self.doc = Some(doc);
                self.dirty = false;
                ctx.activate_plugin("core.canvas");
                ctx.toast(format!("Opened canvas: {rel_path}"));
            }
            Err(e) => ctx.toast(format!("Failed to open canvas: {e}")),
        }
    }

    pub fn save_canvas(&mut self, ctx: &mut PluginContext) {
        if let (Some(path), Some(doc)) = (&self.active_path, &self.doc) {
            if let Err(e) = ctx.vault.save_canvas(path, doc) {
                ctx.toast(format!("Failed to save canvas: {e}"));
            } else {
                self.dirty = false;
            }
        }
    }

    pub fn create_canvas(&mut self, folder: Option<&str>, name: &str, ctx: &mut PluginContext) -> Result<String, mybrain_core::Error> {
        let clean_name = name.trim().trim_end_matches(".canvas");
        let safe_name = if clean_name.is_empty() { "Untitled" } else { clean_name };
        let rel_path = match folder {
            Some(f) if !f.is_empty() => format!("{f}/{safe_name}.canvas"),
            _ => format!("{safe_name}.canvas"),
        };
        let mut doc = CanvasFile::new();
        doc.add_text_node("New Idea\nType Markdown or [[Links]] here…", 0.0, 0.0, 220.0, 140.0);
        ctx.vault.save_canvas(&rel_path, &doc)?;
        self.open_canvas(&rel_path, ctx);
        Ok(rel_path)
    }
}

impl Default for CanvasPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for CanvasPlugin {
    fn id(&self) -> &'static str {
        "core.canvas"
    }

    fn name(&self) -> &'static str {
        "Visual Canvas"
    }

    fn description(&self) -> &'static str {
        "Infinite 2D visual canvas for cards, notes, diagrams, and directional connections"
    }

    fn view_type(&self) -> PluginViewType {
        PluginViewType::MainWorkspace
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        None
    }

    fn on_disable(&mut self, ctx: &mut PluginContext) {
        if self.dirty {
            self.save_canvas(ctx);
        }
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                id: "open_canvas".into(),
                title: "Open Visual Canvas View".into(),
                shortcut: None,
                plugin_id: "core.canvas",
            },
            PluginCommand {
                id: "new_canvas".into(),
                title: "New Visual Canvas".into(),
                shortcut: None,
                plugin_id: "core.canvas",
            },
        ]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        match command_id {
            "open_canvas" => {
                ctx.activate_plugin("core.canvas");
            }
            "new_canvas" => {
                let _ = self.create_canvas(None, "Diagram", ctx);
            }
            _ => {}
        }
    }

    fn on_event(&mut self, event: &AppEvent, ctx: &mut PluginContext) {
        if let AppEvent::RequestOpenCanvas(path) = event {
            self.open_canvas(path, ctx);
        }
    }

    fn render_main_view(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());

        if self.doc.is_none() {
            if let Some(first) = ctx.vault.list_canvas_files().first().cloned() {
                self.open_canvas(&first, ctx);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading(RichText::new("Visual 2D Canvas").size(22.0).color(t.text));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Create interactive visual diagrams, mind maps, and note connections.")
                            .color(t.text_dim)
                            .size(13.5),
                    );
                    ui.add_space(20.0);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("+ Create New Canvas").strong().color(Color32::WHITE))
                                .fill(t.accent)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                        )
                        .clicked()
                    {
                        let _ = self.create_canvas(None, "Diagram", ctx);
                    }
                });
                return;
            }
        }

        let Some(ref mut doc) = self.doc else {
            return;
        };

        let (response_rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(response_rect);
        let center = response_rect.center();

        let zoom = self.state.zoom;
        let pan = self.state.pan;

        let to_screen = |wx: f32, wy: f32| -> Pos2 {
            pos2(center.x + (wx + pan.x) * zoom, center.y + (wy + pan.y) * zoom)
        };
        let to_world = |pos: Pos2| -> (f32, f32) {
            (
                (pos.x - center.x) / zoom - pan.x,
                (pos.y - center.y) / zoom - pan.y,
            )
        };

        // Draw Canvas Background and Grid
        painter.rect_filled(response_rect, CornerRadius::ZERO, t.canvas);
        let grid_size = 40.0 * zoom;
        if grid_size >= 12.0 {
            let offset_x = (center.x + pan.x * zoom).rem_euclid(grid_size);
            let offset_y = (center.y + pan.y * zoom).rem_euclid(grid_size);

            let mut x = response_rect.left() + offset_x;
            while x < response_rect.right() {
                let mut y = response_rect.top() + offset_y;
                while y < response_rect.bottom() {
                    painter.circle_filled(pos2(x, y), 1.0, t.separator.gamma_multiply(0.4));
                    y += grid_size;
                }
                x += grid_size;
            }
        }

        // Panning and Zooming
        let mut is_panning = false;
        let pointer = ui.input(|i| i.pointer.clone());
        if pointer.button_down(PointerButton::Middle)
            || (pointer.button_down(PointerButton::Primary) && ui.input(|i| i.key_down(egui::Key::Space)))
        {
            is_panning = true;
            let delta = ui.input(|i| i.pointer.delta());
            self.state.pan += delta / zoom;
        }

        if response_rect.contains(pointer.hover_pos().unwrap_or_default()) {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta.abs() > 0.0 {
                let factor = (scroll_delta * 0.002).exp();
                let new_zoom = (zoom * factor).clamp(0.2, 3.0);
                if let Some(mouse) = pointer.hover_pos() {
                    let (wx, wy) = to_world(mouse);
                    self.state.pan.x = (mouse.x - center.x) / new_zoom - wx;
                    self.state.pan.y = (mouse.y - center.y) / new_zoom - wy;
                }
                self.state.zoom = new_zoom;
            }
        }

        let mut note_to_open = None;
        let new_edge: Option<(String, String)> = None;
        let mut node_moved = None;
        let mut delete_node_id: Option<String> = None;
        let mut add_text_at: Option<(f32, f32)> = None;
        let mut add_file_at: Option<(String, f32, f32)> = None;

        // Render Edges
        for edge in &doc.edges {
            if let (Some(from_n), Some(to_n)) = (doc.find_node(&edge.from_node), doc.find_node(&edge.to_node)) {
                let (fx, fy) = from_n.pos();
                let (fw, fh) = from_n.size();
                let (tx, ty) = to_n.pos();
                let (tw, th) = to_n.size();
                let p1 = to_screen(fx + fw * 0.5, fy + fh * 0.5);
                let p2 = to_screen(tx + tw * 0.5, ty + th * 0.5);

                let color = if self.state.selected_node.as_ref() == Some(&edge.from_node)
                    || self.state.selected_node.as_ref() == Some(&edge.to_node)
                {
                    t.accent
                } else {
                    t.accent_text.gamma_multiply(0.65)
                };

                painter.line_segment([p1, p2], Stroke::new(2.0 * zoom.clamp(0.5, 2.0), color));
            }
        }

        // Render Nodes
        for node in &doc.nodes {
            let (nx, ny) = node.pos();
            let (nw, nh) = node.size();
            let screen_rect = Rect::from_min_size(
                to_screen(nx, ny),
                vec2(nw * zoom, nh * zoom),
            );

            let is_sel = self.state.selected_node.as_deref() == Some(node.id());
            let frame = if is_sel {
                theme::card_frame(&t, true).stroke(Stroke::new(2.0, t.accent))
            } else {
                theme::card_frame(&t, false)
            };

            painter.rect(
                screen_rect,
                CornerRadius::same(t.radius as u8),
                frame.fill,
                frame.stroke,
                egui::StrokeKind::Inside,
            );

            match node {
                CanvasNode::Text { id, text, .. } => {
                    let text_rect = screen_rect.shrink(10.0 * zoom);
                    let galley = theme::truncated_galley(
                        ui,
                        text,
                        FontId::proportional((12.5 * zoom).clamp(8.0, 18.0)),
                        t.text,
                        text_rect.width(),
                    );
                    painter.galley(text_rect.min, galley, t.text);

                    let node_resp = ui.interact(screen_rect, Id::unique(id), Sense::click_and_drag());
                    if node_resp.clicked() {
                        self.state.selected_node = Some(id.clone());
                    }
                    if node_resp.dragged() {
                        let delta = node_resp.drag_delta();
                        node_moved = Some((id.clone(), nx + delta.x / zoom, ny + delta.y / zoom));
                    }
                }
                CanvasNode::File { id, file, .. } => {
                    let header_rect = Rect::from_min_size(screen_rect.min, vec2(screen_rect.width(), 28.0 * zoom));
                    painter.rect_filled(header_rect, CornerRadius::same(t.radius_sm as u8), t.field);

                    let label = file.trim_end_matches(".md");
                    painter.text(
                        header_rect.left_center() + vec2(8.0, 0.0),
                        Align2::LEFT_CENTER,
                        format!("📄 {label}"),
                        FontId::proportional((11.5 * zoom).clamp(8.0, 16.0)),
                        t.accent_text,
                    );

                    let node_resp = ui.interact(screen_rect, Id::unique(id), Sense::click_and_drag());
                    if node_resp.clicked() {
                        self.state.selected_node = Some(id.clone());
                        if let Ok(note_id) = NoteId::new(label) {
                            note_to_open = Some(note_id);
                        }
                    }
                    if node_resp.dragged() {
                        let delta = node_resp.drag_delta();
                        node_moved = Some((id.clone(), nx + delta.x / zoom, ny + delta.y / zoom));
                    }
                }
                _ => {}
            }
        }

        // Canvas Toolbar Overlay (Top-Center)
        let bar_rect = Rect::from_center_size(
            pos2(response_rect.center().x, response_rect.top() + 32.0),
            vec2(340.0, 42.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
            Frame::NONE
                .fill(t.card.gamma_multiply(0.95))
                .stroke(Stroke::new(1.0, t.card_border))
                .corner_radius(CornerRadius::same(t.radius as u8))
                .inner_margin(Margin::symmetric(12, 6))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: t.shadow,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if theme::ghost_button(ui, &t, RichText::new("+ Text Card").color(t.accent_text)).clicked() {
                            let (wx, wy) = to_world(center);
                            add_text_at = Some((wx, wy));
                        }
                        ui.separator();
                        if theme::ghost_button(ui, &t, RichText::new("+ Note Card").color(t.text)).clicked() {
                            self.state.note_picker_open = true;
                        }
                        ui.separator();
                        let zoom_pct = (self.state.zoom * 100.0).round() as i32;
                        ui.label(RichText::new(format!("{zoom_pct}%")).color(t.text_dim).size(11.0));
                        if theme::ghost_button(ui, &t, RichText::new("Reset").color(t.text_faint)).clicked() {
                            self.state.pan = Vec2::ZERO;
                            self.state.zoom = 1.0;
                        }
                    });
                });
        });

        // Delete key on selected node
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            if let Some(ref sel) = self.state.selected_node {
                delete_node_id = Some(sel.clone());
            }
        }

        // Note Card Picker Modal
        if self.state.note_picker_open {
            let modal = egui::Modal::new(Id::unique("canvas_note_picker_modal"))
                .backdrop_color(Color32::from_black_alpha(150))
                .frame(theme::modal_frame(&t));

            let mut note_selected = None;
            let mut close_modal = false;

            modal.show(ui.ctx(), |ui| {
                ui.set_width(420.0);
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Add Note to Canvas").color(t.text).strong().size(15.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::icon_close_button(ui, &t).clicked() {
                            close_modal = true;
                        }
                    });
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    theme::icon_search(ui, 13.0, t.text_dim);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.state.note_picker_query)
                            .hint_text("Search note to embed…")
                            .desired_width(ui.available_width()),
                    );
                });

                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    let query = self.state.note_picker_query.to_lowercase();
                    for note in ctx.vault.notes() {
                        if query.is_empty() || note.id.as_str().to_lowercase().contains(&query) {
                            let dot = theme::note_kind_color(note.id.as_str(), &t);
                            if theme::list_row(ui, &t, false, Some(dot), note.id.name(), note.id.folder()).clicked() {
                                note_selected = Some(note.id.as_str().to_owned());
                                close_modal = true;
                            }
                        }
                    }
                });
            });

            if let Some(file_path) = note_selected {
                let (wx, wy) = to_world(center);
                add_file_at = Some((file_path, wx, wy));
            }

            if close_modal {
                self.state.note_picker_open = false;
                self.state.note_picker_query.clear();
            }
        }

        // Apply mutations
        if let Some((id, nx, ny)) = node_moved {
            if let Some(node) = doc.find_node_mut(&id) {
                node.set_pos(nx, ny);
                self.dirty = true;
            }
        }

        if let Some((from_id, to_id)) = new_edge {
            doc.add_edge(from_id, to_id, None);
            self.dirty = true;
        }

        if let Some(id) = delete_node_id {
            doc.remove_node(&id);
            self.dirty = true;
        }

        if let Some((wx, wy)) = add_text_at {
            let id = doc.add_text_node("New Idea\nType Markdown or [[Links]] here…", wx - 100.0, wy - 60.0, 220.0, 140.0);
            self.state.selected_node = Some(id);
            self.dirty = true;
        }

        if let Some((file_path, wx, wy)) = add_file_at {
            let id = doc.add_file_node(file_path, wx - 120.0, wy - 80.0, 260.0, 160.0);
            self.state.selected_node = Some(id);
            self.dirty = true;
        }

        if let Some(id) = note_to_open {
            ctx.open_note(id);
            ctx.activate_plugin("core.markdown");
        }

        if self.dirty {
            self.save_canvas(ctx);
        }

        if is_panning {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
        }
    }
}
