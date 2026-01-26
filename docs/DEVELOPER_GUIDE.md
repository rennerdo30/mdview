# mdview - Developer Guide

This guide provides technical documentation for developers working on or contributing to mdview.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Project Structure](#project-structure)
3. [Core Systems](#core-systems)
4. [Key Design Patterns](#key-design-patterns)
5. [Performance Optimizations](#performance-optimizations)
6. [Testing](#testing)
7. [Debugging](#debugging)

---

## Architecture Overview

mdview is built using **egui/eframe** for the GUI and follows an **immediate mode** UI architecture.

### Technology Stack

| Component | Library | Version | Purpose |
|-----------|---------|---------|---------|
| GUI | egui/eframe | 0.29 | Immediate mode GUI |
| Markdown | pulldown-cmark | 0.12 | Markdown parsing |
| File Watch | notify | 7.0 | File system events |
| Config | toml/serde | 0.8/1.0 | Configuration |
| PDF | printpdf | 0.7 | PDF generation |
| Syntax | syntect | 5.2 | Syntax highlighting |
| Plugins | mlua | 0.10 | Lua scripting |

### Core Principles

1. **Immediate Mode UI**: No retained state in widgets; rebuilt every frame
2. **Performance First**: Sub-50ms startup, <2ms frame time
3. **Modular Design**: Feature-gated optional components
4. **Data-Driven**: TOML configuration and JSON annotations

---

## Project Structure

```
mdview/
├── src/
│   ├── main.rs               # Entry point, CLI parsing
│   ├── lib.rs                # Library exports
│   ├── app/
│   │   ├── mod.rs            # Module exports
│   │   ├── state.rs          # Application state
│   │   └── viewer.rs         # Main eframe::App impl
│   ├── config/
│   │   ├── mod.rs            # Module exports
│   │   ├── schema.rs         # Serde structs
│   │   ├── loader.rs         # Config discovery/loading
│   │   └── defaults.rs       # Default values
│   ├── markdown/
│   │   ├── mod.rs            # Module exports
│   │   ├── parser.rs         # pulldown-cmark wrapper
│   │   ├── renderer.rs       # Event-to-egui conversion
│   │   └── cache.rs          # Render caching
│   ├── toc/
│   │   ├── mod.rs            # Module exports
│   │   ├── builder.rs        # TOC extraction
│   │   └── panel.rs          # TOC sidebar UI
│   ├── annotations/
│   │   ├── mod.rs            # Module exports
│   │   ├── model.rs          # Data structures
│   │   ├── storage.rs        # JSON persistence
│   │   └── ui.rs             # Annotation UI
│   ├── export/
│   │   ├── mod.rs            # Module exports
│   │   └── pdf.rs            # PDF export
│   ├── theme/
│   │   ├── mod.rs            # Module exports
│   │   ├── style.rs          # egui Style generation
│   │   └── builtin.rs        # Built-in themes
│   ├── watcher/
│   │   ├── mod.rs            # Module exports
│   │   └── file_watcher.rs   # notify integration
│   └── plugin/
│       ├── mod.rs            # Module exports
│       ├── lua_runtime.rs    # Lua environment
│       └── api.rs            # Plugin API
├── themes/                   # Built-in TOML themes
├── plugins/                  # Example plugins
├── docs/                     # Documentation
└── tests/                    # Integration tests
```

---

## Core Systems

### Application State (`app/state.rs`)

The `AppState` struct holds all mutable application state:

```rust
pub struct AppState {
    pub current_file: Option<PathBuf>,
    pub content: String,
    pub toc: TocTree,
    pub annotations: AnnotationStore,
    pub config: Config,
    // ... more fields
}
```

Key methods:
- `load_file()` - Load and parse markdown file
- `reload_file()` - Hot reload preserving scroll
- `save_annotations()` - Persist annotations

### Markdown Renderer (`markdown/renderer.rs`)

Converts pulldown-cmark events to egui widgets:

```rust
impl MarkdownRenderer {
    pub fn render(
        &mut self,
        ui: &mut Ui,
        events: &[Event<'_>],
        annotations: &AnnotationStore,
        heading_positions: &mut Vec<f32>,
        config: &Config,
    ) { ... }
}
```

Event handling flow:
1. `handle_start_tag()` - Open tags, track state
2. `handle_text()` - Accumulate text content
3. `handle_end_tag()` - Close tags, render content

### TOC Builder (`toc/builder.rs`)

Extracts headings during parse pass:

```rust
pub fn build_toc(content: &str) -> TocTree {
    // Parse markdown
    // Extract H1-H6 headings
    // Build tree structure
    // Return flat + tree views
}
```

### File Watcher (`watcher/file_watcher.rs`)

Uses notify with debouncing:

```rust
impl FileWatcher {
    pub fn new(
        path: PathBuf,
        sender: Sender<FileEvent>,
        ctx: Context,
    ) -> Result<Self, WatcherError> { ... }
}
```

Debounce: 100ms to avoid rapid re-renders.

---

## Key Design Patterns

### Event-Driven Rendering

No intermediate AST - parse directly to events, render to egui:

```
Markdown String
    ↓ pulldown-cmark
Event Stream
    ↓ MarkdownRenderer
egui Widgets
```

### Feature Gates

Optional features minimize binary size:

```toml
[features]
default = ["syntax-highlighting"]
syntax-highlighting = ["syntect"]
plugins = ["mlua"]
```

### Configuration Layering

1. Built-in defaults
2. User config file
3. CLI arguments

```rust
let config = config::loader::load_default()?;
// Override with CLI args
if args.no_toc { config.general.show_toc = false; }
```

---

## Performance Optimizations

### 1. Viewport Culling

Only render visible content in ScrollArea:

```rust
egui::ScrollArea::vertical()
    .auto_shrink([false, false])
    .show(ui, |ui| {
        // Content rendered here
    });
```

### 2. Render Caching

LRU cache for layout jobs (max 500 entries):

```rust
pub struct RenderCache {
    entries: HashMap<u64, CacheEntry>,
    max_entries: usize,
}
```

### 3. Debounced Watching

100ms debounce prevents rapid file change events:

```rust
const WATCHER_DEBOUNCE_MS: u64 = 100;
```

### 4. Release Profile

Optimized release builds:

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific module
cargo test markdown::

# With output
cargo test -- --nocapture
```

### Test Coverage

Key test areas:
- `markdown/parser.rs` - Parse correctness
- `toc/builder.rs` - TOC extraction
- `annotations/model.rs` - Annotation operations
- `theme/style.rs` - Color parsing

### Adding Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = "...";

        // Act
        let result = function(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

---

## Debugging

### Enable Logging

```bash
RUST_LOG=debug cargo run -- file.md
RUST_LOG=mdview=trace cargo run -- file.md
```

### egui Debug

```rust
// In viewer.rs
ui.ctx().debug_on_hover();
```

### Performance Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Run profiler
cargo flamegraph -- file.md
```

### Common Issues

| Issue | Solution |
|-------|----------|
| Blank window | Check file path, enable logging |
| Slow render | Check cache stats, profile |
| Crash on load | Check file encoding (UTF-8) |
| Theme not applied | Verify TOML syntax |

---

## Next Steps

- Read [ARCHITECTURE.md](ARCHITECTURE.md) for system design
- Read [PLUGIN_SDK.md](PLUGIN_SDK.md) for plugin development
- Read [CONFIGURATION.md](CONFIGURATION.md) for config options
