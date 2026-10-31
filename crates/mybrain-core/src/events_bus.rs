//! Decoupled message bus for communication between shell and plugins.

use std::collections::VecDeque;
use chrono::NaiveDate;
use crate::note::NoteId;

/// Application-wide events emitted by the shell or plugins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// Request to open a specific note in the editor.
    RequestOpenNote(NoteId),
    /// Emitted when a note's content is modified in memory.
    NoteModified(NoteId),
    /// Emitted when a note has been saved to disk.
    NoteSaved(NoteId),
    /// Emitted when a note has been deleted from the vault.
    NoteDeleted(NoteId),
    /// Emitted when a note has been renamed.
    NoteRenamed { old: NoteId, new: NoteId },
    /// Request to switch the active main workspace plugin view.
    ActivatePlugin(String),
    /// Request to toggle a left side panel plugin view (open or close).
    ToggleSidePanel(String),
    /// Request to toggle the right panel plugin view (open or close).
    ToggleRightPanel(String),
    /// Request to open a modal overlay by ID.
    OpenModal(String),
    /// Insert text into the active editor at the current cursor position.
    InsertText(String),
    /// Display a transient status toast notification in the shell status bar.
    StatusMessage(String),
    /// Request to navigate to a daily note for a given date.
    RequestOpenDaily(NaiveDate),
    /// Request to open a visual canvas file.
    RequestOpenCanvas(String),
    /// Request to show the new note dialog, optionally targeting a folder.
    RequestNewNote(Option<String>),
}

/// A decoupled FIFO event bus allowing plugins and shell to communicate without direct references.
#[derive(Debug, Default, Clone)]
pub struct EventBus {
    queue: VecDeque<AppEvent>,
}

impl EventBus {
    /// Create a new, empty event bus.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Post an event to the queue.
    pub fn emit(&mut self, event: AppEvent) {
        self.queue.push_back(event);
    }

    /// Drain all pending events from the queue for processing.
    pub fn drain(&mut self) -> Vec<AppEvent> {
        self.queue.drain(..).collect()
    }

    /// Returns `true` if there are no pending events.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns the number of queued events.
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_emit_and_drain() {
        let mut bus = EventBus::new();
        assert!(bus.is_empty());

        let id = NoteId::new("TestNote").unwrap();
        bus.emit(AppEvent::RequestOpenNote(id.clone()));
        bus.emit(AppEvent::InsertText("Hello".into()));

        assert_eq!(bus.len(), 2);
        assert!(!bus.is_empty());

        let events = bus.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], AppEvent::RequestOpenNote(id));
        assert_eq!(events[1], AppEvent::InsertText("Hello".into()));
        assert!(bus.is_empty());
    }
}
