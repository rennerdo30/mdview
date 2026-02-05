# mdview - Architecture

This document describes the high-level architecture and design decisions of mdview.

## Table of Contents

1. [System Overview](#system-overview)
2. [Component Architecture](#component-architecture)
3. [Data Flow](#data-flow)
4. [State Management](#state-management)
5. [Rendering Pipeline](#rendering-pipeline)
6. [Extension Points](#extension-points)
7. [Design Decisions](#design-decisions)

---

## System Overview

mdview is designed as a high-performance, extensible markdown viewer with these core principles:

- **Fast Startup**: Target <50ms cold start
- **Smooth Rendering**: Target <2ms frame time
- **Minimal Dependencies**: Pure Rust where possible
- **Extensibility**: TOML themes, Lua plugins

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         mdview Application                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────────┐ │
│  │   CLI Layer  │  │  Config Layer │  │    Plugin Layer      │ │
│  │   (clap)     │  │  (toml/serde) │  │    (mlua)            │ │
│  └──────┬───────┘  └───────┬───────┘  └──────────┬───────────┘ │
│         │                  │                      │             │
│         └──────────────────┼──────────────────────┘             │
│                            ▼                                    │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Application Core                       │  │
│  │  ┌────────────┐  ┌─────────────┐  ┌────────────────────┐ │  │
│  │  │  AppState  │  │   Viewer    │  │  FileWatcher       │ │  │
│  │  │  - file    │  │  (eframe)   │  │  (notify)          │ │  │
│  │  │  - content │  │             │  │                    │ │  │
│  │  │  - config  │  │             │  │                    │ │  │
│  │  └────────────┘  └─────────────┘  └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────┘  │
│                            │                                    │
│         ┌──────────────────┼──────────────────┐                │
│         ▼                  ▼                  ▼                 │
│  ┌────────────┐  ┌─────────────────┐  ┌────────────────────┐   │
│  │  Markdown  │  │   TOC System    │  │   Annotations      │   │
│  │  - parser  │  │   - builder     │  │   - model          │   │
│  │  - render  │  │   - panel       │  │   - storage        │   │
│  │  - cache   │  │                 │  │   - ui             │   │
│  └────────────┘  └─────────────────┘  └────────────────────┘   │
│         │                  │                  │                 │
│         └──────────────────┼──────────────────┘                │
│                            ▼                                    │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Output Layer                           │  │
│  │  ┌─────────────────┐  ┌─────────────────────────────────┐│  │
│  │  │  egui Renderer  │  │      PDF Export (printpdf)      ││  │
│  │  └─────────────────┘  └─────────────────────────────────┘│  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Architecture

### Core Components

| Component | Responsibility | Dependencies |
|-----------|---------------|--------------|
| `app::state` | Application state management | config |
| `app::viewer` | eframe::App implementation | all modules |
| `markdown::parser` | Markdown parsing | pulldown-cmark |
| `markdown::renderer` | Event-to-egui conversion | egui |
| `toc::builder` | Heading extraction | markdown::parser |
| `toc::panel` | TOC sidebar UI | egui |
| `annotations::model` | Annotation data structures | serde |
| `annotations::storage` | JSON persistence | serde_json |
| `export::pdf` | PDF generation | printpdf |
| `theme::style` | egui Style creation | egui |
| `watcher` | File change detection | notify |
| `plugin` | Lua scripting | mlua |

### Module Dependencies

```
main.rs
    └── app::viewer
            ├── app::state
            │       ├── config
            │       ├── toc::builder
            │       └── annotations::storage
            ├── markdown::renderer
            │       └── markdown::parser
            ├── toc::panel
            ├── annotations::ui
            ├── export::pdf
            ├── theme::style
            └── watcher::file_watcher
```

---

## Data Flow

### File Loading

```
User Action (CLI/Menu/DragDrop)
         │
         ▼
┌─────────────────┐
│  AppState       │
│  load_file()    │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌───────┐  ┌──────────────┐
│ Read  │  │ Parse TOC    │
│ File  │  │ (toc::build) │
└───┬───┘  └──────┬───────┘
    │             │
    ▼             ▼
┌───────────────────────────┐
│      AppState Updated     │
│  - content: String        │
│  - toc: TocTree           │
│  - content_hash: String   │
└───────────────────────────┘
         │
         ▼
┌─────────────────┐
│ Load Annotations│
│ (if exists)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Request Repaint │
└─────────────────┘
```

### Rendering Pipeline

```
eframe::App::update()
         │
         ▼
┌─────────────────────────────┐
│  Handle Events              │
│  - File watcher events      │
│  - Keyboard shortcuts       │
│  - Drag and drop           │
└────────────┬────────────────┘
             │
             ▼
┌─────────────────────────────┐
│  Render Menu Bar            │
│  egui::TopBottomPanel::top  │
└────────────┬────────────────┘
             │
             ▼
┌─────────────────────────────┐
│  Render Status Bar          │
│  egui::TopBottomPanel::bottom│
└────────────┬────────────────┘
             │
             ▼
┌─────────────────────────────┐
│  Render TOC Sidebar         │
│  egui::SidePanel::left      │
│  (if visible)               │
└────────────┬────────────────┘
             │
             ▼
┌─────────────────────────────┐
│  Render Main Content        │
│  egui::CentralPanel         │
│                             │
│  ┌───────────────────────┐  │
│  │ Parse Markdown        │  │
│  │ (pulldown-cmark)      │  │
│  └───────────┬───────────┘  │
│              │              │
│  ┌───────────▼───────────┐  │
│  │ Render Events         │  │
│  │ (MarkdownRenderer)    │  │
│  │  - Headings           │  │
│  │  - Paragraphs         │  │
│  │  - Code blocks        │  │
│  │  - Lists              │  │
│  │  - Links              │  │
│  └───────────────────────┘  │
└─────────────────────────────┘
```

### Hot Reload Flow

```
File System Change
         │
         ▼
┌─────────────────┐
│  notify crate   │
│  (debounced)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  FileEvent::    │
│  Modified       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Channel send   │
│  (mpsc)         │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────┐
│  handle_file_events()       │
│  - Collect events           │
│  - reload_file()            │
│  - Preserve scroll position │
│  - Set status message       │
└─────────────────────────────┘
         │
         ▼
┌─────────────────┐
│ Request Repaint │
└─────────────────┘
```

---

## State Management

### AppState Structure

```rust
pub struct AppState {
    // File state
    pub current_file: Option<PathBuf>,
    pub content: String,
    pub content_hash: String,

    // Parsed data
    pub toc: TocTree,
    pub annotations: AnnotationStore,

    // UI state
    pub toc_visible: bool,
    pub toc_width: f32,
    pub scroll_offset: f32,
    pub current_heading_idx: Option<usize>,
    pub heading_positions: Vec<f32>,

    // Configuration
    pub config: Config,

    // Watcher channels
    pub file_event_rx: Option<Receiver<FileEvent>>,
    pub file_event_tx: Option<Sender<FileEvent>>,

    // Caching
    pub render_cache: RenderCache,

    // Transient UI state
    pub status_message: Option<(String, Instant)>,
}
```

### State Updates

State is updated through methods on `AppState`:

| Method | Purpose |
|--------|---------|
| `load_file()` | Load new file |
| `reload_file()` | Hot reload |
| `save_annotations()` | Persist annotations |
| `set_status()` | Show status message |
| `clear_expired_status()` | Remove old messages |

---

## Rendering Pipeline

### Immediate Mode Rendering

egui uses immediate mode - UI rebuilt every frame:

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // 1. Handle events
    self.handle_file_events();
    self.handle_keyboard_shortcuts(ctx);

    // 2. Render UI (rebuilt every frame)
    self.render_menu_bar(ctx);
    self.render_status_bar(ctx);
    self.render_toc_sidebar(ctx);
    self.render_main_content(ctx);
}
```

### Markdown Rendering

Two-pass approach:

1. **Parse Pass**: pulldown-cmark events
2. **Render Pass**: Convert events to egui widgets

```rust
// Parse
let events: Vec<_> = parser::parse(&content).collect();

// Render
renderer.render(ui, &events, &annotations, &mut heading_positions, &config);
```

---

## Extension Points

### Themes (TOML)

Custom themes via TOML files:

```toml
[colors]
background = "#1e1e1e"
text = "#d4d4d4"
# ...
```

### Plugins (Lua)

Feature-gated Lua scripting:

```lua
mdview_hooks.on_file_open = function(filepath)
    mdview.log_info("Opened: " .. filepath)
end
```

### Configuration

User configuration via TOML:

```toml
[general]
theme = "dark"
hot_reload = true
```

---

## Design Decisions

### Why egui/eframe?

| Consideration | egui | Alternatives |
|--------------|------|--------------|
| Startup time | ~20ms | GTK: ~100ms |
| Binary size | ~7MB | Electron: ~100MB |
| Cross-platform | Native | Web: Browser required |
| Rust native | Yes | Most: FFI |

### Why pulldown-cmark?

- 5-7x faster than alternatives
- Streaming parser (low memory)
- CommonMark compliant
- Pure Rust

### Why Immediate Mode?

- No retained widget state
- Simple mental model
- Easy hot reload
- Natural for viewer app

### Why Feature Gates?

- Reduce binary size (syntect adds ~3MB)
- Faster compilation without unused features
- Optional plugin system (mlua adds ~2MB)

### Why JSON for Annotations?

- Human readable
- Easy debugging
- Sidecar files don't modify source
- Standard format

---

## Performance Considerations

### Startup Optimization

1. Lazy load syntax highlighting
2. Parallel config + file read
3. Defer plugin initialization

### Render Optimization

1. Viewport culling (only visible content)
2. LRU cache for layout jobs (image: 50, syntax: 100, mermaid: 200 entries)
3. Debounced file watching
4. `Arc<LayoutJob>` for syntax cache (avoids expensive clones on cache hits)
5. Deterministic FNV-1a config hashing for efficient cache invalidation
6. TOC search results cached (invalidated only when query changes)
7. Status bar file path cached (avoids per-frame string allocation)

### Memory Optimization

1. Stream parsing (no full AST)
2. Cache eviction policy (LRU with configurable max sizes)
3. Clone-on-write for large strings
4. Annotation file size limits (10MB max on load and save)

### Security

1. Image path traversal prevention (canonicalization + base path containment check)
2. Plugin sandbox: removes dangerous globals (`io`, `os`, `debug`, `require`, `load`, `loadstring`, `rawget`, `rawset`, `package`)
3. Directory scanning: depth limit (32 levels), symlink cycle prevention
4. Windows file association: exe path validation against injection characters
5. Network operations: timeout enforcement (10s for update checks)
