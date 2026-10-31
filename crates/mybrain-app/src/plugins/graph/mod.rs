//! Obsidian-style interactive 2D knowledge graph plugin for Brainwash.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use egui::{
    pos2, vec2, Align2, CornerRadius, CursorIcon, FontId, Frame, Margin, Pos2, Rect, RichText,
    Sense, Stroke, Ui, UiBuilder, Vec2,
};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::NoteId;

use crate::plugins::{
    MyBrainPlugin, PluginCommand, PluginContext, PluginViewType, RibbonButton, RibbonSlot,
};
use crate::theme::Tokens;

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: NoteId,
    pub title: String,
    pub pos: Pos2,
    pub vel: Vec2,
    pub radius: f32,
    pub degree: usize,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: NoteId,
    pub to: NoteId,
}

#[derive(Debug, Clone)]
pub struct GraphState {
    pub nodes: HashMap<NoteId, GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub pan: Vec2,
    pub zoom: f32,
    pub dragged_node: Option<NoteId>,
    pub hovered_node: Option<NoteId>,
    pub initialized: bool,
    pub last_signature: u64,
    pub local_graph_only: bool,
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            pan: Vec2::ZERO,
            zoom: 1.0,
            dragged_node: None,
            hovered_node: None,
            initialized: false,
            last_signature: 0,
            local_graph_only: false,
        }
    }
}

impl GraphState {
    pub fn sync(
        &mut self,
        vault: &mybrain_core::vault::Vault,
        link_index: &mybrain_core::LinkIndex,
        active_note: Option<&NoteId>,
    ) {
        let mut hasher = DefaultHasher::new();
        self.local_graph_only.hash(&mut hasher);
        if self.local_graph_only {
            active_note.hash(&mut hasher);
        }
        for note in vault.notes() {
            note.id.hash(&mut hasher);
            for out in link_index.outgoing(&note.id) {
                out.resolved.hash(&mut hasher);
            }
        }
        let signature = hasher.finish();
        if self.initialized && signature == self.last_signature {
            return;
        }

        self.last_signature = signature;
        let mut new_nodes = HashMap::new();
        let mut new_edges = Vec::new();
        let all_note_ids: Vec<NoteId> = vault.notes().map(|n| n.id.clone()).collect();

        let allowed_ids: Option<HashSet<NoteId>> = if self.local_graph_only {
            active_note.map(|center_id| {
                let mut set = HashSet::new();
                set.insert(center_id.clone());
                for out in link_index.outgoing(center_id) {
                    if let Some(target) = &out.resolved {
                        set.insert(target.clone());
                    }
                }
                for in_link in link_index.backlinks(center_id) {
                    set.insert(in_link.source.clone());
                }
                set
            })
        } else {
            None
        };

        for (i, id) in all_note_ids.iter().enumerate() {
            if let Some(allowed) = &allowed_ids {
                if !allowed.contains(id) {
                    continue;
                }
            }

            let degree = link_index.backlinks(id).len() + link_index.outgoing(id).len();
            let radius = (6.0 + (degree as f32).sqrt() * 3.5).clamp(7.0, 22.0);

            let (pos, vel) = if let Some(old) = self.nodes.get(id) {
                (old.pos, old.vel)
            } else {
                let angle = (i as f32) * 2.399_963_1; // golden ratio spacing
                let r = 80.0 + 15.0 * (i as f32).sqrt();
                (pos2(r * angle.cos(), r * angle.sin()), Vec2::ZERO)
            };

            new_nodes.insert(
                id.clone(),
                GraphNode {
                    id: id.clone(),
                    title: id.name().to_string(),
                    pos,
                    vel,
                    radius,
                    degree,
                },
            );
        }

        for note in vault.notes() {
            if let Some(allowed) = &allowed_ids {
                if !allowed.contains(&note.id) {
                    continue;
                }
            }

            for out in link_index.outgoing(&note.id) {
                if let Some(target) = &out.resolved {
                    if let Some(allowed) = &allowed_ids {
                        if !allowed.contains(target) {
                            continue;
                        }
                    }

                    if new_nodes.contains_key(&note.id) && new_nodes.contains_key(target) {
                        new_edges.push(GraphEdge {
                            from: note.id.clone(),
                            to: target.clone(),
                        });
                    }
                }
            }
        }

        self.nodes = new_nodes;
        self.edges = new_edges;
        self.initialized = true;
    }

    pub fn step_simulation(&mut self) {
        let node_ids: Vec<NoteId> = self.nodes.keys().cloned().collect();
        let k_repulsion = 3800.0;
        let k_spring = 0.035;
        let rest_length = 90.0;
        let damping = 0.82;
        let center_gravity = 0.008;

        let mut forces: HashMap<NoteId, Vec2> = node_ids.iter().map(|id| (id.clone(), Vec2::ZERO)).collect();

        // 1. Repulsion between all node pairs
        for i in 0..node_ids.len() {
            let id_a = &node_ids[i];
            let pos_a = self.nodes[id_a].pos;

            for id_b in &node_ids[i + 1..] {
                let pos_b = self.nodes[id_b].pos;
                let delta = pos_a - pos_b;
                let dist_sq = delta.length_sq().max(16.0);
                let dist = dist_sq.sqrt();
                let force_mag = (k_repulsion / dist_sq).min(30.0);
                let force_vec = (delta / dist) * force_mag;

                if let Some(f_a) = forces.get_mut(id_a) {
                    *f_a += force_vec;
                }
                if let Some(f_b) = forces.get_mut(id_b) {
                    *f_b -= force_vec;
                }
            }

            if let Some(f_a) = forces.get_mut(id_a) {
                *f_a -= pos_a.to_vec2() * center_gravity;
            }
        }

        // 2. Attraction along edges
        for edge in &self.edges {
            if let (Some(a), Some(b)) = (self.nodes.get(&edge.from), self.nodes.get(&edge.to)) {
                let delta = b.pos - a.pos;
                let dist = delta.length().max(1.0);
                let force_mag = (dist - rest_length) * k_spring;
                let force_vec = (delta / dist) * force_mag;

                if let Some(f_from) = forces.get_mut(&edge.from) {
                    *f_from += force_vec;
                }
                if let Some(f_to) = forces.get_mut(&edge.to) {
                    *f_to -= force_vec;
                }
            }
        }

        // 3. Update positions and velocities
        for (id, force) in forces {
            if Some(&id) == self.dragged_node.as_ref() {
                continue;
            }
            if let Some(node) = self.nodes.get_mut(&id) {
                node.vel = (node.vel + force) * damping;
                let speed = node.vel.length();
                if speed > 18.0 {
                    node.vel = (node.vel / speed) * 18.0;
                }
                node.pos += node.vel;
            }
        }
    }
}

pub struct GraphPlugin {
    pub state: GraphState,
}

impl GraphPlugin {
    pub fn new() -> Self {
        Self {
            state: GraphState::default(),
        }
    }
}

impl Default for GraphPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for GraphPlugin {
    fn id(&self) -> &'static str {
        "core.graph"
    }

    fn name(&self) -> &'static str {
        "Graph View"
    }

    fn description(&self) -> &'static str {
        "Interactive 2D knowledge graph showing notes and connections across your vault"
    }

    fn ribbon_button(&self) -> Option<RibbonButton> {
        Some(RibbonButton {
            icon: "🕸",
            tooltip: "Graph View (Ctrl+3)",
            slot: RibbonSlot::Top,
            view_type: PluginViewType::MainWorkspace,
            priority: 60,
        })
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            id: "open_graph".into(),
            title: "Open Graph View".into(),
            shortcut: Some("Ctrl+3".into()),
            plugin_id: "core.graph",
        }]
    }

    fn execute_command(&mut self, command_id: &str, ctx: &mut PluginContext) {
        if command_id == "open_graph" {
            ctx.activate_plugin("core.graph");
        }
    }

    fn on_event(&mut self, event: &AppEvent, _ctx: &mut PluginContext) {
        match event {
            AppEvent::NoteModified(_)
            | AppEvent::NoteSaved(_)
            | AppEvent::NoteDeleted(_)
            | AppEvent::NoteRenamed { .. } => {
                self.state.initialized = false;
            }
            _ => {}
        }
    }

    fn render_main_view(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        let t = Tokens::get(ui.ctx());
        self.state.sync(ctx.vault, ctx.index, ctx.active_note);
        self.state.step_simulation();

        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());
        let center = rect.center();
        let painter = ui.painter_at(rect);

        // Background canvas grid
        painter.rect_filled(rect, CornerRadius::ZERO, t.canvas);

        let to_screen = |pos: Pos2, pan: Vec2, zoom: f32| -> Pos2 {
            center + pan + (pos.to_vec2() * zoom)
        };
        let from_screen = |pos: Pos2, pan: Vec2, zoom: f32| -> Pos2 {
            Pos2::new(
                (pos.x - center.x - pan.x) / zoom,
                (pos.y - center.y - pan.y) / zoom,
            )
        };

        // Pan and Zoom
        let mut is_dragging = false;
        if response.dragged_by(egui::PointerButton::Primary) || response.dragged_by(egui::PointerButton::Secondary) {
            is_dragging = true;
            if let Some(dragged_id) = &self.state.dragged_node {
                if let Some(ptr_pos) = response.interact_pointer_pos() {
                    let world_pos = from_screen(ptr_pos, self.state.pan, self.state.zoom);
                    if let Some(node) = self.state.nodes.get_mut(dragged_id) {
                        node.pos = world_pos;
                        node.vel = Vec2::ZERO;
                    }
                }
            } else {
                self.state.pan += response.drag_delta();
            }
        } else {
            self.state.dragged_node = None;
        }

        // Scroll zoom
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta.abs() > 0.0 {
                let factor = (scroll_delta * 0.002).exp();
                self.state.zoom = (self.state.zoom * factor).clamp(0.15, 3.5);
            }
        }

        // Detect node hover / drag target
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        let mut hovered_node = None;
        if let Some(ptr) = pointer_pos {
            if rect.contains(ptr) {
                for (id, node) in &self.state.nodes {
                    let screen_pos = to_screen(node.pos, self.state.pan, self.state.zoom);
                    let hit_radius = (node.radius * self.state.zoom).max(12.0);
                    if ptr.distance(screen_pos) <= hit_radius {
                        hovered_node = Some(id.clone());
                        break;
                    }
                }
            }
        }
        self.state.hovered_node = hovered_node.clone();

        if response.drag_started() {
            if let Some(h) = &hovered_node {
                self.state.dragged_node = Some(h.clone());
            }
        }

        // 1. Draw Edges
        for edge in &self.state.edges {
            if let (Some(a), Some(b)) = (self.state.nodes.get(&edge.from), self.state.nodes.get(&edge.to)) {
                let p1 = to_screen(a.pos, self.state.pan, self.state.zoom);
                let p2 = to_screen(b.pos, self.state.pan, self.state.zoom);

                let is_highlighted = hovered_node.as_ref().map_or(false, |h| *h == edge.from || *h == edge.to)
                    || ctx.active_note.map_or(false, |an| *an == edge.from || *an == edge.to);

                let color = if is_highlighted {
                    t.accent_text.gamma_multiply(0.85)
                } else if hovered_node.is_some() {
                    t.separator.gamma_multiply(0.2)
                } else {
                    t.separator.gamma_multiply(0.65)
                };

                let stroke_w = if is_highlighted { 1.8 } else { 1.0 };
                painter.line_segment([p1, p2], Stroke::new(stroke_w, color));
            }
        }

        // 2. Draw Nodes
        let mut clicked_node = None;
        for (id, node) in &self.state.nodes {
            let screen_pos = to_screen(node.pos, self.state.pan, self.state.zoom);
            let r = (node.radius * self.state.zoom).clamp(4.0, 48.0);
            let is_hovered = hovered_node.as_ref() == Some(id);
            let is_active = ctx.active_note == Some(id);

            let is_connected = if let Some(h) = &hovered_node {
                self.state.edges.iter().any(|e| (&e.from == h && &e.to == id) || (&e.to == h && &e.from == id))
            } else {
                false
            };

            let base_color = if is_active {
                t.accent
            } else if node.degree > 3 {
                t.success
            } else {
                t.accent_text
            };

            let node_color = if is_hovered || is_active {
                base_color
            } else if hovered_node.is_some() && !is_connected {
                base_color.gamma_multiply(0.25)
            } else {
                base_color.gamma_multiply(0.85)
            };

            if is_hovered || is_active {
                painter.circle_filled(screen_pos, r + 4.0 * self.state.zoom.clamp(0.8, 2.0), t.accent_soft);
            }

            painter.circle_filled(screen_pos, r, node_color);
            painter.circle_stroke(screen_pos, r, Stroke::new(1.0, t.card));

            let show_label = self.state.zoom > 0.75 || is_hovered || is_active || is_connected;
            if show_label {
                let label_color = if is_hovered || is_active {
                    t.text
                } else if hovered_node.is_some() && !is_connected {
                    t.text_faint.gamma_multiply(0.3)
                } else {
                    t.text_dim
                };

                let font_size = (11.0 * self.state.zoom).clamp(9.0, 14.0);
                painter.text(
                    screen_pos + vec2(0.0, r + 3.0),
                    Align2::CENTER_TOP,
                    &node.title,
                    FontId::proportional(font_size),
                    label_color,
                );
            }
        }

        // Node click handling: open note & jump to Markdown editor
        if response.clicked() {
            if let Some(h) = hovered_node {
                clicked_node = Some(h);
            }
        }

        if let Some(id) = clicked_node {
            ctx.open_note(id);
            ctx.activate_plugin("core.markdown");
        }

        // 3. Floating HUD Overlay
        let hud_rect = Rect::from_min_size(
            pos2(rect.left() + 16.0, rect.top() + 16.0),
            vec2(280.0, 48.0),
        );

        ui.scope_builder(UiBuilder::new().max_rect(hud_rect), |ui| {
            Frame::NONE
                .fill(t.card.gamma_multiply(0.92))
                .stroke(Stroke::new(1.0, t.card_border))
                .corner_radius(CornerRadius::same(t.radius as u8))
                .inner_margin(Margin::symmetric(14, 8))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: t.shadow,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Knowledge Graph").strong().size(13.5).color(t.text));
                        ui.add_space(4.0);
                        let node_count = self.state.nodes.len();
                        let edge_count = self.state.edges.len();
                        crate::theme::pill(ui, &format!("{node_count} notes"), t.accent_text, t.accent_soft);
                        crate::theme::pill(ui, &format!("{edge_count} links"), t.text_dim, t.field);
                    });
                });
        });

        // Top-right controls overlay: Local Graph toggle, Zoom & Reset
        let controls_rect = Rect::from_min_size(
            pos2(rect.right() - 290.0, rect.top() + 16.0),
            vec2(274.0, 42.0),
        );

        ui.scope_builder(UiBuilder::new().max_rect(controls_rect), |ui| {
            Frame::NONE
                .fill(t.card.gamma_multiply(0.92))
                .stroke(Stroke::new(1.0, t.card_border))
                .corner_radius(CornerRadius::same(t.radius as u8))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let zoom_pct = (self.state.zoom * 100.0).round() as i32;
                        ui.label(RichText::new(format!("{zoom_pct}%")).color(t.text_dim).size(11.5));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::theme::ghost_button(ui, &t, RichText::new("Reset View").color(t.accent_text).size(11.5)).clicked() {
                                self.state.pan = Vec2::ZERO;
                                self.state.zoom = 1.0;
                            }
                            let local_label = if self.state.local_graph_only {
                                "Local (1-hop) ●"
                            } else {
                                "Local (1-hop)"
                            };
                            let local_col = if self.state.local_graph_only {
                                t.accent_text
                            } else {
                                t.text_dim
                            };
                            if crate::theme::ghost_button(ui, &t, RichText::new(local_label).color(local_col).size(11.5))
                                .on_hover_text("Toggle between full vault graph and active note's 1-hop neighborhood")
                                .clicked()
                            {
                                self.state.local_graph_only = !self.state.local_graph_only;
                            }
                        });
                    });
                });
        });

        // Hover tooltip with connection counts
        if let Some(h) = &self.state.hovered_node {
            if let Some(node) = self.state.nodes.get(h) {
                let screen_pos = to_screen(node.pos, self.state.pan, self.state.zoom);
                let in_count = ctx.index.backlinks(h).len();
                let out_count = ctx.index.outgoing(h).len();

                let tooltip_rect = Rect::from_min_size(
                    pos2(screen_pos.x + 12.0, screen_pos.y - 28.0),
                    vec2(180.0, 48.0),
                );

                ui.scope_builder(UiBuilder::new().max_rect(tooltip_rect), |ui| {
                    Frame::NONE
                        .fill(t.card)
                        .stroke(Stroke::new(1.0, t.accent_border))
                        .corner_radius(CornerRadius::same(t.radius_sm as u8))
                        .inner_margin(Margin::symmetric(10, 6))
                        .shadow(egui::Shadow {
                            offset: [0, 4],
                            blur: 12,
                            spread: 0,
                            color: t.shadow,
                        })
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&node.title).strong().size(12.5).color(t.text));
                                ui.label(
                                    RichText::new(format!("{in_count} incoming · {out_count} outgoing"))
                                        .size(10.5)
                                        .color(t.text_faint),
                                );
                            });
                        });
                });
            }
        }

        if response.hovered() {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
        }
        if is_dragging {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
        }
    }
}
