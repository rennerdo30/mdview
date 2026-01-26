# mdview Technical Specification

## Overview

mdview is a high-performance, cross-platform markdown viewer built in Rust using egui/eframe for the GUI. It prioritizes fast startup times, smooth rendering, and extensibility.

## Performance Targets

| Metric | Target |
|--------|--------|
| Cold start | < 50ms |
| Frame time | < 2ms (500+ FPS capable) |
| Memory (1MB doc) | < 50MB |
| Hot reload | < 100ms |

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                        MdViewApp                             │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐   │
│  │  AppState   │  │ MarkdownRenderer│  │   TocPanel      │   │
│  │  - file     │  │ - parse      │  │   - entries      │   │
│  │  - content  │  │ - render     │  │   - collapsed    │   │
│  │  - toc      │  │ - cache      │  │   - current      │   │
│  └─────────────┘  └──────────────┘  └───────────────────┘   │
│         │                │                    │              │
│         └────────────────┼────────────────────┘              │
│                          │                                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                    Config                              │   │
│  │  - general (theme, hot_reload, toc)                   │   │
│  │  - markdown (tables, strikethrough, tasks)            │   │
│  │  - annotations, export, keybindings                   │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **File Loading**: File → Parser → Events → TOC Builder → State
2. **Rendering**: Events → Renderer → egui widgets → Screen
3. **Hot Reload**: FileWatcher → Event → Reload → Re-render
4. **Annotations**: Selection → Create → Store → Persist

## Modules

### `app`
- `state.rs`: Application state management (file, content, TOC, annotations)
- `viewer.rs`: Main `eframe::App` implementation

### `config`
- `schema.rs`: Serde structs for TOML configuration
- `loader.rs`: Config discovery and loading
- `defaults.rs`: Default values and constants

### `markdown`
- `parser.rs`: pulldown-cmark wrapper with custom options
- `renderer.rs`: Event-to-egui conversion
- `cache.rs`: LRU render cache for performance

### `toc`
- `builder.rs`: Extract headings, build tree structure
- `panel.rs`: Sidebar UI with collapsible entries

### `annotations`
- `model.rs`: Annotation types (Highlight, Note, Bookmark)
- `storage.rs`: JSON sidecar persistence
- `ui.rs`: Annotation popup and rendering

### `export`
- `pdf.rs`: PDF export via printpdf

### `theme`
- `style.rs`: egui Style generation
- `builtin.rs`: Dark, light, sepia themes

### `watcher`
- `file_watcher.rs`: notify integration with debouncing

### `plugin` (feature-gated)
- `lua_runtime.rs`: Sandboxed Lua environment
- `api.rs`: Plugin API and hooks

## File Formats

### Configuration (`config.toml`)

```toml
[general]
theme = "dark"
hot_reload = true
show_toc = true
toc_width = 250

[window]
width = 1000
height = 700

[markdown]
tables = true
strikethrough = true
task_lists = true
syntax_highlighting = true

[annotations]
enabled = true
auto_save = true

[export]
pdf_theme = "light"
include_toc = true
page_size = "A4"

[keybindings]
toggle_toc = "Ctrl+T"
export_pdf = "Ctrl+P"
reload = "F5"
```

### Annotations (`.mdview-annotations.json`)

```json
{
  "annotations": {
    "ann_12345": {
      "id": "ann_12345",
      "kind": "highlight",
      "start": 100,
      "end": 150,
      "color": "#ffeb3b",
      "created_at": 1700000000,
      "updated_at": 1700000000
    }
  },
  "document_hash": "abc123...",
  "version": 1
}
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+O | Open file |
| Ctrl+T | Toggle TOC |
| Ctrl+P | Export PDF |
| F5 | Reload file |
| Ctrl+Q | Quit |
| Escape | Cancel annotation |

## Feature Flags

```toml
[features]
default = ["syntax-highlighting"]
syntax-highlighting = ["syntect"]
plugins = ["mlua"]
```

## Performance Optimizations

1. **Lazy Loading**: Syntax highlighting and plugins loaded on demand
2. **Viewport Culling**: Only render visible content
3. **Render Caching**: LRU cache for layout jobs
4. **Debounced Watching**: 100ms debounce on file changes
5. **Release Profile**: LTO, single codegen unit, stripped binary

## Testing

```bash
# Unit tests
cargo test

# With all features
cargo test --all-features

# Benchmarks
cargo bench
```

## Building

```bash
# Development
cargo build

# Release (optimized)
cargo build --release

# With plugins
cargo build --release --features plugins
```
