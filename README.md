# MyBrain 🧠

A fast, maintainable Obsidian clone in Rust built with [egui](https://github.com/emilk/egui) / [eframe](https://crates.io/crates/eframe).

---

## Highlights

- **Double-Bracket `[[Wiki Links]]`**:
  - Full support for `[[Note]]`, `[[Folder/Note]]`, `[[Note|Alias]]`, and `[[Note#Heading|Alias]]`.
  - Case-insensitive, shortest-path resolution matching Obsidian's algorithm.
  - Interactive auto-completion popup when typing `[[` in the editor.
  - Live rendered markdown with clickable links that instantly open or create notes.
  - Incoming **Backlinks** panel with line context, **Outgoing Links**, and **Unresolved Link** creator.
  - Automatic link rewriting across the whole vault when renaming notes!
- **Daily Notes**:
  - One dedicated note per calendar day (`Daily/YYYY-MM-DD.md`).
  - Configurable template supporting `{{date}}`, `{{date:FORMAT}}`, and `{{title}}`.
  - Quick-switch buttons for *Today*, *Yesterday*, *Tomorrow*, and full historical timeline.
- **Quick Notes (Fleeting Capture)**:
  - Instant thought capture dialog via **Ctrl+Q** or the toolbar button.
  - Automatically saves as an independent note in `Quick Notes/` and appends a timestamped link to today's daily note.
- **Calendar & Events**:
  - Events are stored as standard Markdown notes with front-matter (`type: event`, `date`, `start`, `end`, `location`).
  - Plain Markdown and fully compatible with Obsidian & Dataview plugins.
  - Dedicated Agenda view showing upcoming events, all-day badges, and direct note links.
- **Markdown Editor & Live Preview**:
  - Three view modes: **Edit Only**, **Split View**, and **Preview Only**.
  - Syntax-highlighted code editor for Markdown and `[[wiki links]]`.
  - Interactive task checkboxes (`- [ ]` / `- [x]`): clicking a checkbox in preview mode toggles it in the Markdown file!
  - Headings, blockquotes, code blocks, lists, tags (`#tag`), and URLs.
- **Command Palette & Quick Switcher**:
  - Press **Ctrl+P** or **Ctrl+O** to fuzzy search notes by title or content and jump immediately.

---

## Architecture & Maintainability

The codebase is structured as a clean Cargo workspace separating pure domain logic from the graphical user interface:

```text
mybrain/
├── Cargo.toml               # Workspace configuration
├── shell.nix                # NixOS / Linux environment with graphics libraries
├── crates/
│   ├── mybrain-core/        # UI-agnostic domain engine (zero GUI dependencies)
│   │   ├── src/
│   │   │   ├── lib.rs       # Re-exports and module documentation
│   │   │   ├── error.rs     # Unified Error type with path context
│   │   │   ├── note.rs      # Note and validated NoteId newtype
│   │   │   ├── frontmatter.rs # Minimal YAML front-matter parser/formatter
│   │   │   ├── links.rs     # [[Wiki link]] parser & autocompletion helper
│   │   │   ├── index.rs     # Graph index: forward links, backlinks, unresolved
│   │   │   ├── vault.rs     # Vault operations (atomic writes, renames, search)
│   │   │   ├── daily.rs     # Daily notes generator and timeline queries
│   │   │   ├── quick.rs     # Quick capture thought store
│   │   │   ├── events.rs    # Calendar event models and queries
│   │   │   └── markdown.rs  # Line-oriented Markdown subset parser
│   │   └── tests/           # 36 unit tests covering parsing, links, and vault
│   │
│   └── mybrain-app/         # Desktop application built with egui 0.37
│       └── src/
│           ├── main.rs      # eframe App loop, keyboard shortcuts, panel layout
│           ├── theme.rs     # Obsidian dark theme visuals and color palette
│           ├── state.rs     # Centralized reactive application state
│           └── views/
│               ├── editor.rs         # Markdown editor + live preview
│               ├── sidebar.rs        # Note explorer, search, and action bar
│               ├── daily_view.rs     # Daily notes date navigator
│               ├── events_view.rs    # Calendar agenda view
│               ├── quick_capture.rs  # Floating quick capture dialog
│               ├── backlinks.rs      # Backlinks & connections panel
│               ├── command_palette.rs# Quick switcher (Ctrl+P)
│               └── dialogs.rs        # New Note / New Event dialogs
```

### Why this design is easy to maintain
1. **Zero UI leak in domain logic**: `mybrain-core` contains all knowledge graph algorithms, link resolution, and file mutations. It can be tested in milliseconds without running a windowing system.
2. **Atomic Disk Operations**: Vault writes use atomic temporary file swaps so files never corrupt on crash.
3. **Decoupled View Components**: Each UI view in `mybrain-app/src/views/` is a pure function taking `(&mut Ui, &mut AppState)` without retaining internal GUI state.
4. **Obsidian Compatibility**: Vault notes are stored as plain Markdown `.md` files on your disk. You can open the exact same vault folder in Obsidian or git-track it without proprietary databases.

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + S` | Save current note |
| `Ctrl + P` or `Ctrl + O` | Open Command Palette / Quick Switcher |
| `Ctrl + Q` | Open Quick Capture dialog |
| `Ctrl + N` | Create a new note |
| `Alt + ←` | Navigate history back |
| `Alt + →` | Navigate history forward |
| `Escape` | Close active dialog / modal |

---

## Running the Application

### Under Nix / NixOS:
```bash
nix-shell
cargo run -p mybrain-app
```

### On standard Linux / macOS / Windows:
Ensure Rust (1.85+) is installed, then run:
```bash
cargo run -p mybrain-app
```

### Running Tests:
```bash
cargo test
```
All 36 tests run in parallel and pass with zero warnings.
