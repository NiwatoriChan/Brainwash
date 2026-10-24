# Brainwash

A blazing-fast, local-first personal knowledge base built in Rust with [egui](https://github.com/emilk/egui) and [eframe](https://crates.io/crates/eframe).

---

## Inspiration & Philosophy

**Brainwash is inspired by [Obsidian](https://obsidian.md/), but customized directly to personal workflow and needs.**

Obsidian proved that plain Markdown files and bidirectional `[[wiki links]]` form the ideal foundation for personal knowledge management. However, Obsidian is built on Electron, often takes seconds to boot, requires dozens of third-party community plugins just to get basic daily journaling, calendar agendas, and quick note capture working, and carries significant memory and maintenance overhead.

**Brainwash re-engineers this experience from the ground up in Rust:**
* **Instant Native Performance**: Boots in milliseconds with negligible RAM usage and zero web-engine overhead.
* **Everything Built-in**: Daily notes, quick fleeting thought capture, calendar agenda, and backlinks are first-class native features—no plugins required.
* **100% Obsidian Compatible**: Notes are stored as standard `.md` files in a local vault directory. You can open, sync, or git-track the exact same folder in Obsidian without modification.
* **PhotoCraft Visual Aesthetics**: Features the design system and curated palettes from [PhotoCraft](https://github.com/storytold/photocraft), with thin floating overlay scrollbars, sleek typography, and zero missing-font tofu glitches.

---

## Features

### 1. Inspired by Obsidian
* **Bidirectional `[[Wiki Links]]`**:
  * Full syntax support for `[[Note]]`, `[[Folder/Note]]`, `[[Note|Alias]]`, and `[[Note#Heading|Alias]]`.
  * Shortest-path, case-insensitive resolution matching Obsidian's algorithm.
  * Interactive auto-completion popup that triggers as soon as you type `[[` in the editor.
  * Live clickable links in preview mode that open existing notes or create new ones on the fly.
  * Vault-wide link rewriting: renaming a note automatically updates all incoming `[[links]]` across every file.
* **Connections & Backlinks Panel**:
  * **Incoming Backlinks**: shows all notes that reference the active document, complete with context snippets.
  * **Outgoing Links**: lists all targets referenced by the current note.
  * **Unresolved Links Explorer**: surfaces broken or pending references with one-click creation.
* **Markdown Editor & Live Split View**:
  * Seamlessly toggle between **Edit Only**, **Split View**, and **Preview Only**.
  * Interactive task checkboxes (`- [ ]` / `- [x]`): clicking a checkbox in preview mode toggles it in the Markdown file on disk.
  * Syntax formatting for headings, quotes, code blocks, lists, tags, and URLs.
* **Command Palette / Quick Switcher (`Ctrl+P` / `Ctrl+O`)**:
  * Fast fuzzy search across note titles and content to jump anywhere in milliseconds.

### 2. Custom to My Needs
* **Built-in Daily Notes Engine**:
  * One dedicated note per calendar day (`Daily/YYYY-MM-DD.md`).
  * Automatic template expansion supporting `{{date}}`, `{{date:FORMAT}}`, and `{{title}}`.
  * Day-by-day navigation (`← Previous Day` / `Next Day →`) and an instant `Today` jump button.
* **Quick Note Capture (`Ctrl+Q`)**:
  * Floating quick capture modal designed for fleeting thoughts, tasks, and snippets without losing your active context.
  * Automatically saves into `Quick Notes/` and logs a timestamped bidirectional link into today's Daily Note.
* **Integrated Calendar & Events Agenda**:
  * Events are stored directly in human-readable Markdown frontmatter (`type: event`, `date`, `start`, `end`, `location`).
  * Dedicated agenda view with color-coded dates, time badges, and direct note links—eliminating the need for heavy calendar plugins like Dataview or Full Calendar.
* **PhotoCraft Design System & Themes**:
  * Polished dark and light themes adapted from [PhotoCraft](https://github.com/storytold/photocraft):
    * **Studio (Dark)** *(Default)*: Near-black `#141415` chrome, `#1A1A1C` cards, and Studio violet `#8B7CF6` accents.
    * **Studio (Light)**: Clean, low-glare `#F6F6F8` chrome with purple accents.
    * **Pro (Charcoal)**: Photoshop-style `#323232` flat charcoal panels with electric blue accents.
    * **Solarized Dark**: Ethan Schoonover's classic base03/base02 palette.
    * **Adwaita Dark**: GNOME libadwaita dark palette.
  * Smooth macOS-style floating scrollbars (`ScrollStyle::thin()`) and curated corner radii.
  * Clean typographic badges (`[D]`, `[E]`, `[Q]`, `•`) and fallback font support to ensure crisp rendering with zero tofu boxes.

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + S` | Save current note |
| `Ctrl + P` or `Ctrl + O` | Command Palette / Quick Switcher |
| `Ctrl + Q` | Quick Capture modal (fleeting thoughts) |
| `Ctrl + N` | Create a new note |
| `Alt + ←` | Navigate history back |
| `Alt + →` | Navigate history forward |
| `Escape` | Close active modal or dialog |

---

## Architecture & Maintainability

The codebase is organized into a clean Cargo workspace separating pure domain logic from graphical user interface code:

```text
mybrain/
├── Cargo.toml               # Workspace configuration
├── shell.nix                # NixOS / Linux environment with graphics libraries
├── run.sh                   # One-click launcher script
├── crates/
│   ├── mybrain-core/        # Pure UI-agnostic domain engine (zero GUI dependencies)
│   │   ├── src/
│   │   │   ├── lib.rs       # Crate re-exports and module docs
│   │   │   ├── error.rs     # Contextual error handling
│   │   │   ├── note.rs      # Note and validated NoteId newtype
│   │   │   ├── frontmatter.rs # Clean YAML front-matter parser & writer
│   │   │   ├── links.rs     # [[Wiki link]] parser & autocompletion helper
│   │   │   ├── index.rs     # Graph index: forward links, backlinks, unresolved
│   │   │   ├── vault.rs     # Vault store (atomic writes, renames, search)
│   │   │   ├── daily.rs     # Daily notes generator & template engine
│   │   │   ├── quick.rs     # Quick capture engine & daily note logger
│   │   │   ├── events.rs    # Event models and agenda store
│   │   │   └── markdown.rs  # Markdown subset parser & task toggler
│   │   └── tests/           # 36 unit tests covering parsing, links, and vault
│   │
│   └── mybrain-app/         # Desktop application built with egui / eframe
│       └── src/
│           ├── main.rs      # Window lifecycle, top navigation, keyboard shortcuts
│           ├── theme.rs     # PhotoCraft design tokens, font loader, and styles
│           ├── state.rs     # Centralized reactive application state
│           └── views/
│               ├── editor.rs         # Markdown editor + split preview
│               ├── sidebar.rs        # Note explorer, search, and badges
│               ├── daily_view.rs     # Daily notes timeline navigator
│               ├── events_view.rs    # Calendar agenda view
│               ├── quick_capture.rs  # Floating quick capture dialog
│               ├── backlinks.rs      # Connections & backlinks panel
│               ├── command_palette.rs# Quick switcher (Ctrl+P)
│               └── dialogs.rs        # New Note / New Event dialogs
```

### Architectural Highlights
1. **Zero UI Leak in Domain Engine**: `mybrain-core` contains all knowledge graph algorithms, link resolution, and file mutations. It compiles rapidly and is validated by automated unit tests without spawning windows.
2. **Atomic Disk Operations**: File saves utilize atomic temporary file replacement so note contents are never corrupted if power is lost or a crash occurs.
3. **Decoupled View Functions**: Every view in `views/` is a pure function taking `(&mut Ui, &mut AppState)` without retaining internal GUI state.

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
cargo test
```
All 36 domain unit tests run in parallel and pass with zero warnings.

---

## Vault Storage
By default, notes are stored in `~/BrainwashVault/`. To back up or sync your notes, simply initialize a git repository inside that directory or sync it with any file sync service.
