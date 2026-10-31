# Brainwash

A blazing-fast, local-first personal knowledge base and graph built in Rust with [egui](https://github.com/emilk/egui) and [eframe](https://crates.io/crates/eframe).

---

## Inspiration & Philosophy

**Brainwash is inspired by [Obsidian](https://obsidian.md/), but customized directly to personal workflow and needs.**

Obsidian proved that plain Markdown files and bidirectional `[[wiki links]]` form the ideal foundation for personal knowledge management. However, Obsidian is built on Electron, often takes seconds to boot, carries heavy memory overhead, and requires dozens of third-party community plugins just to get basic daily journaling, calendar agendas, and quick note capture working.

**Brainwash re-engineers this experience from the ground up in Rust:**
* **Instant Native Performance**: Boots in milliseconds with negligible RAM usage and zero web-engine overhead.
* **Modular Plugin Architecture**: Every feature view—from the File Explorer and Markdown Editor to Visual Canvas, Graph, and Backlinks—is an isolated, self-contained plugin.
* **Runtime Enable/Disable**: Toggle any feature plugin dynamically on the fly in Settings without restarting the application.
* **100% Obsidian Compatible**: Notes are stored as standard `.md` files and `.canvas` files in a local vault directory. Open, sync, or git-track the exact same folder in Obsidian seamlessly.
* **Dedicated Three-Pane Layout**: Always-visible left ribbon bar, collapsible left navigation explorer, central workspace, and collapsible right connections panel.
* **PhotoCraft Visual Aesthetics**: Features the design system and curated palettes from [PhotoCraft](https://github.com/storytold/photocraft), with thin floating overlay scrollbars, sleek typography, and zero missing-font tofu glitches.

---

## Features

### 1. Everything is a Plugin
Brainwash is architected around a unified `MyBrainPlugin` trait managed by a central `PluginRegistry` and a decoupled `EventBus`:

1. **File Explorer (`core.explorer`)** *(Left Side Panel)*:
   - Hierarchical folder and note tree with instant real-time search filtering.
   - Custom folder emojis, colored folder accents, and note deletion/renaming context menus.
   - Drag-and-drop payload support for dragging notes into canvas or editor.
2. **Markdown Notes & Tabs (`core.markdown`)** *(Main Workspace)*:
   - Seamlessly toggle between **Edit Only**, **Split View**, and **Preview Only** (`Ctrl+E`).
   - Multi-tab document bar with history navigation (`←` / `→`), `Ctrl+W` close tab, and `Ctrl+Tab` cycle.
   - Interactive task checkboxes (`- [ ]` / `- [x]`): clicking a checkbox in preview mode toggles it in the Markdown file on disk.
   - Bidirectional `[[Wiki Links]]` auto-completion popup that triggers as you type `[[`.
   - Vault-wide link rewriting: renaming a note automatically updates all incoming `[[links]]` across every file.
   - Auto-flushes any dirty buffers to disk when disabled or switched.
3. **Connections & Backlinks (`core.backlinks`)** *(Right Side Panel)*:
   - **Incoming Backlinks**: Inspect all notes that reference the active document, complete with context snippets.
   - **Outgoing Links**: Lists all wiki-links referenced by the current document.
   - **Unresolved Links**: Surfaces broken or pending references with one-click note creation.
   - Housed in a dedicated, resizable right panel (`Ctrl+Shift+B` or top-bar button) for side-by-side editing.
4. **Visual Canvas (`core.canvas`)** *(Main Workspace)*:
   - Infinite 2D whiteboard fully compatible with Obsidian JSON Canvas format (`.canvas`).
   - Cards, text nodes, colors, and interactive directional connections.
   - Automatically activates when clicking any `.canvas` file in the File Explorer.
5. **Interactive Knowledge Graph (`core.graph`)** *(Main Workspace)*:
   - 2D force-directed physics graph visualizing connections across your entire vault.
   - Color-coded nodes (Daily notes, Events, Quick notes, standard notes) with hover connection tooltips.
   - Click any node to open that note directly in the Markdown editor.
6. **Daily Notes Engine (`core.daily`)** *(Main Workspace)*:
   - One dedicated note per calendar day (`Daily/YYYY-MM-DD.md`).
   - Automatic template expansion supporting `{{date}}`, `{{date:FORMAT}}`, and `{{title}}`.
   - Day-by-day timeline navigation (`← Previous Day` / `Next Day →`) and an instant `Today` jump button.
7. **Quick Note Capture (`core.quick_note`)** *(Main Workspace & Modal)*:
   - Google Keep-style masonry card grid for viewing and pinning fleeting thoughts.
   - Floating quick capture modal (`Ctrl+Q`) to jot ideas down without losing your active context.
   - Automatically saves into `Quick Notes/` and logs a timestamped bidirectional link into today's Daily Note.
8. **Events Agenda (`core.events`)** *(Main Workspace)*:
   - Events stored directly in Markdown frontmatter (`type: event`, `date`, `start`, `end`, `location`).
   - Dedicated agenda view with color-coded dates, time badges, and direct note links.
9. **Emoji & Icon Picker (`core.emoji`)** *(Modal Overlay)*:
   - Searchable categorized emoji catalog with instant search filtering.
   - Set custom icons and palette colors on folders and notes.

### 2. Modern Shell & Navigation
* **Permanent Left Ribbon Bar (`shell/ribbon.rs`)**:
  - Always-visible vertical icon ribbon on the far left.
  - Top stack: Quick view switches for enabled plugins with active indicator highlights.
  - Clean & uncluttered: File-backed views (`core.markdown`, `core.canvas`) and the Connections panel are opened via files, shortcuts, and top-bar controls rather than cluttering the ribbon bar.
  - Bottom stack: Quick access to Command Palette (`Ctrl+P`), Vault Switcher (`Ctrl+O`), and Settings / Plugin Manager (`Ctrl+,`).
* **Collapsible Three-Pane Layout**:
  - Left panel: File Explorer (`Ctrl+B`).
  - Center panel: Main workspace (Markdown, Daily, Canvas, Graph, Events, Quick Notes) with fallback card.
  - Right panel: Connections & Backlinks panel (`Ctrl+Shift+B` or header toggle).
* **PhotoCraft Design System & Themes**:
  - **Studio (Dark)** *(Default)*: Near-black `#141415` chrome, `#1A1A1C` cards, and Studio violet accents.
  - **Studio (Light)**: Clean, low-glare `#F6F6F8` chrome with purple accents.
  - **Pro (Charcoal)**: Flat charcoal panels with electric blue accents.
  - **Solarized Dark**: Ethan Schoonover's classic base03/base02 palette.
  - **Adwaita Dark**: GNOME libadwaita dark palette.

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + S` | Save current note |
| `Ctrl + P` | Command Palette & fuzzy search |
| `Ctrl + O` | Open / Switch Vault |
| `Ctrl + N` | Create a new note |
| `Ctrl + Q` | Quick Capture modal (fleeting thoughts) |
| `Ctrl + B` or `Ctrl + \` | Toggle Left Sidebar (File Explorer) |
| `Ctrl + Shift + B` | Toggle Right Panel (Connections & Backlinks) |
| `Ctrl + E` | Toggle Markdown view mode (Edit / Preview / Split) |
| `Ctrl + W` | Close current editor tab |
| `Ctrl + Tab` | Cycle to next editor tab |
| `Ctrl + ,` | Preferences & Plugin Manager |
| `Ctrl + 1` | Switch to Markdown Notes workspace |
| `Ctrl + 2` | Switch to Daily Notes workspace |
| `Ctrl + 3` | Switch to Graph View workspace |
| `Alt + ←` / `Alt + →` | Navigate note history backward / forward |
| `Escape` | Close active modal, palette, or dialog |

---

## Architecture & Codebase Layout

The workspace cleanly separates domain logic from GUI code and plugin implementations:

```text
Brainwash/
├── Cargo.toml               # Workspace configuration
├── shell.nix                # NixOS / Linux environment with graphics libraries
├── run.sh                   # One-click launcher script
├── crates/
│   ├── mybrain-core/        # Pure domain engine (zero GUI dependencies)
│   │   ├── src/
│   │   │   ├── lib.rs       # Crate re-exports
│   │   │   ├── error.rs     # Error types
│   │   │   ├── note.rs      # Note model & validated NoteId newtype
│   │   │   ├── frontmatter.rs # YAML frontmatter parser & writer
│   │   │   ├── links.rs     # [[Wiki link]] parser & autocompletion helper
│   │   │   ├── index.rs     # Knowledge graph index (forward links, backlinks)
│   │   │   ├── vault.rs     # Local filesystem vault (atomic writes, renames)
│   │   │   ├── daily.rs     # Daily notes generator & template engine
│   │   │   ├── quick.rs     # Quick capture engine & daily note logger
│   │   │   ├── events.rs    # Event models and agenda store
│   │   │   ├── canvas.rs    # Obsidian JSON Canvas data structures & serde
│   │   │   ├── markdown.rs  # Markdown block/inline parser & task toggler
│   │   │   ├── events_bus.rs# Decoupled AppEvent message bus
│   │   │   └── settings.rs  # AppSettings with plugin enable states & configs
│   │   └── tests/           # 45 domain unit tests
│   │
│   └── mybrain-app/         # Desktop application built with egui / eframe
│       └── src/
│           ├── main.rs      # Window setup, native options, shell bootstrap
│           ├── theme.rs     # Design tokens, custom font loader, and palettes
│           ├── shell/       # Core UI shell coordinator
│           │   ├── app_shell.rs       # Central orchestrator (panels, events, shortcuts)
│           │   ├── ribbon.rs          # Always-visible vertical icon ribbon bar
│           │   ├── command_palette.rs # Fuzzy-search command runner
│           │   ├── settings_modal.rs  # Settings & runtime Plugin Manager
│           │   ├── vault_switcher.rs  # Recent vaults & vault switcher modal
│           │   └── dialogs.rs         # Shared modals (New Note, Rename, New Folder)
│           └── plugins/     # Self-contained feature plugins (MyBrainPlugin)
│               ├── mod.rs             # Trait definition, PluginRegistry, PluginContext
│               ├── explorer/          # File tree, search, folder actions (SidePanel)
│               ├── markdown/          # Multi-tab editor & preview (MainWorkspace)
│               ├── quick_note/        # Card grid & quick capture modal (MainWorkspace)
│               ├── daily/             # Daily notes navigator (MainWorkspace)
│               ├── canvas/            # 2D infinite visual canvas (MainWorkspace)
│               ├── graph/             # 2D interactive knowledge graph (MainWorkspace)
│               ├── events/            # Calendar agenda view (MainWorkspace)
│               ├── backlinks/         # Connections & backlinks inspector (RightPanel)
│               └── emoji/             # Emoji & color picker modal (ModalOverlay)
```

---

## Getting Started

### Launch with `run.sh`
```bash
./run.sh
```
`run.sh` automatically detects NixOS / Linux environments and loads the required graphics (`libGL`), Wayland, and X11 libraries via `shell.nix`.

### Launch with Cargo
```bash
cargo run -p mybrain-app --bin brainwash
```

### Run Tests
```bash
cargo test --workspace
```
All **57 automated tests** (45 in `mybrain-core` and 12 in `mybrain-app`) run in parallel and pass cleanly with zero warnings.

---

## Vault Storage
By default, notes and canvases are stored in `~/BrainwashVault/`. Because all files are stored as standard Markdown (`.md`) and JSON Canvas (`.canvas`) files, you can open, edit, sync, or git-track your vault with standard tools or Obsidian at any time without conversion.
