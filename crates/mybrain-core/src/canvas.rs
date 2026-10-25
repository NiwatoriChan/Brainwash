//! Obsidian-compatible JSON Canvas specification and data model.
//!
//! Specifications follow the JSON Canvas 1.0 standard (https://jsoncanvas.org/):
//! - Infinite 2D space with `x`, `y`, `width`, `height`.
//! - Supported node types: `text` (Markdown text cards), `file` (embedded notes),
//!   `link` (external URLs), and `group` (visual grouping rectangles).
//! - Edges connecting nodes with directional arrows and optional text labels.

use serde::{Deserialize, Serialize};

/// An Obsidian JSON Canvas document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CanvasFile {
    #[serde(default)]
    pub nodes: Vec<CanvasNode>,
    #[serde(default)]
    pub edges: Vec<CanvasEdge>,
}

/// A node placed on the infinite canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CanvasNode {
    #[serde(rename = "text")]
    Text {
        id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        #[serde(default)]
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    #[serde(rename = "file")]
    File {
        id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        /// Relative path or NoteId (e.g. `"Work/Roadmap"` or `"Daily/2026-10-25"`).
        file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subpath: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    #[serde(rename = "link")]
    Link {
        id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    #[serde(rename = "group")]
    Group {
        id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
}

impl CanvasNode {
    pub fn id(&self) -> &str {
        match self {
            Self::Text { id, .. }
            | Self::File { id, .. }
            | Self::Link { id, .. }
            | Self::Group { id, .. } => id,
        }
    }

    pub fn pos(&self) -> (f32, f32) {
        match self {
            Self::Text { x, y, .. }
            | Self::File { x, y, .. }
            | Self::Link { x, y, .. }
            | Self::Group { x, y, .. } => (*x, *y),
        }
    }

    pub fn size(&self) -> (f32, f32) {
        match self {
            Self::Text { width, height, .. }
            | Self::File { width, height, .. }
            | Self::Link { width, height, .. }
            | Self::Group { width, height, .. } => (*width, *height),
        }
    }

    pub fn set_pos(&mut self, new_x: f32, new_y: f32) {
        match self {
            Self::Text { x, y, .. }
            | Self::File { x, y, .. }
            | Self::Link { x, y, .. }
            | Self::Group { x, y, .. } => {
                *x = new_x;
                *y = new_y;
            }
        }
    }

    pub fn set_size(&mut self, new_width: f32, new_height: f32) {
        match self {
            Self::Text { width, height, .. }
            | Self::File { width, height, .. }
            | Self::Link { width, height, .. }
            | Self::Group { width, height, .. } => {
                *width = new_width;
                *height = new_height;
            }
        }
    }

    pub fn color(&self) -> Option<&str> {
        match self {
            Self::Text { color, .. }
            | Self::File { color, .. }
            | Self::Link { color, .. }
            | Self::Group { color, .. } => color.as_deref(),
        }
    }
}

/// An edge/connector between two nodes on the canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasEdge {
    pub id: String,
    pub from_node: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_end: Option<String>,
    pub to_node: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl CanvasFile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse from JSON string.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to pretty formatted JSON.
    pub fn to_string_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Add a text note card.
    pub fn add_text_node(
        &mut self,
        text: impl Into<String>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> String {
        let id = format!("text_{:08x}", rand_u32());
        self.nodes.push(CanvasNode::Text {
            id: id.clone(),
            x,
            y,
            width,
            height,
            text: text.into(),
            color: None,
        });
        id
    }

    /// Add an embedded file note card.
    pub fn add_file_node(
        &mut self,
        file: impl Into<String>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> String {
        let id = format!("file_{:08x}", rand_u32());
        self.nodes.push(CanvasNode::File {
            id: id.clone(),
            x,
            y,
            width,
            height,
            file: file.into(),
            subpath: None,
            color: None,
        });
        id
    }

    /// Connect two nodes by an edge.
    pub fn add_edge(
        &mut self,
        from_node: impl Into<String>,
        to_node: impl Into<String>,
        label: Option<String>,
    ) -> String {
        let id = format!("edge_{:08x}", rand_u32());
        self.edges.push(CanvasEdge {
            id: id.clone(),
            from_node: from_node.into(),
            from_side: Some("right".into()),
            from_end: None,
            to_node: to_node.into(),
            to_side: Some("left".into()),
            to_end: Some("arrow".into()),
            label,
            color: None,
        });
        id
    }

    /// Remove a node and any connected edges.
    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.retain(|n| n.id() != node_id);
        self.edges
            .retain(|e| e.from_node != node_id && e.to_node != node_id);
    }

    /// Remove an edge by its identifier.
    pub fn remove_edge(&mut self, edge_id: &str) {
        self.edges.retain(|e| e.id != edge_id);
    }

    /// Find node by id.
    pub fn find_node(&self, id: &str) -> Option<&CanvasNode> {
        self.nodes.iter().find(|n| n.id() == id)
    }

    /// Find node mutably by id.
    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut CanvasNode> {
        self.nodes.iter_mut().find(|n| n.id() == id)
    }

    /// Retrieve all note paths referenced by file nodes in this canvas.
    pub fn linked_notes(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .filter_map(|n| match n {
                CanvasNode::File { file, .. } => Some(file.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// Simple pseudorandom number generator for unique node IDs without extra dependencies.
fn rand_u32() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::SystemTime;

    static COUNTER: AtomicU32 = AtomicU32::new(1);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let time_low = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    time_low.wrapping_mul(1664525).wrapping_add(count.wrapping_mul(1013904223))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_roundtrip_serde() {
        let mut canvas = CanvasFile::new();
        let n1 = canvas.add_text_node("# Welcome\nHello world", 100.0, 150.0, 250.0, 180.0);
        let n2 = canvas.add_file_node("Work/Roadmap", 450.0, 150.0, 300.0, 200.0);
        canvas.add_edge(n1, n2, Some("relates to".into()));

        let json = canvas.to_string_pretty().expect("serialization works");
        let parsed: CanvasFile = CanvasFile::parse(&json).expect("deserialization works");

        assert_eq!(parsed.nodes.len(), 2);
        assert_eq!(parsed.edges.len(), 1);
        assert_eq!(parsed.linked_notes(), vec!["Work/Roadmap"]);
    }

    #[test]
    fn test_remove_node_cascades_edges() {
        let mut canvas = CanvasFile::new();
        let n1 = canvas.add_text_node("A", 0.0, 0.0, 100.0, 100.0);
        let n2 = canvas.add_text_node("B", 200.0, 0.0, 100.0, 100.0);
        canvas.add_edge(&n1, &n2, None);

        assert_eq!(canvas.nodes.len(), 2);
        assert_eq!(canvas.edges.len(), 1);

        canvas.remove_node(&n1);
        assert_eq!(canvas.nodes.len(), 1);
        assert_eq!(canvas.edges.len(), 0);
    }
}
