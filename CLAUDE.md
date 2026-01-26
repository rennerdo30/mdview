# mdview Development Guide

This guide helps AI assistants understand and contribute to mdview.

## Project Overview

mdview is a fast, cross-platform markdown viewer built with Rust and egui. Key design principles:

1. **Performance First**: Sub-50ms startup, smooth 60fps rendering
2. **Simplicity**: Minimal dependencies, straightforward architecture
3. **Extensibility**: TOML themes, Lua plugins (optional)

## Directory Structure

```
src/
├── main.rs          # Entry point, CLI parsing
├── lib.rs           # Library exports
├── app/             # Application state and viewer
├── config/          # Configuration loading
├── markdown/        # Parsing and rendering
├── toc/             # Table of contents
├── annotations/     # Highlights, notes, bookmarks
├── export/          # PDF export
├── theme/           # Theme system
├── watcher/         # File watching
└── plugin/          # Lua plugins (feature-gated)
```

## Key Patterns

### State Management
All application state lives in `AppState` (src/app/state.rs). The viewer holds mutable reference and updates on each frame.

### Event-Driven Rendering
Markdown is parsed to pulldown-cmark events, then converted to egui widgets. No intermediate AST.

### Feature Gates
Optional features like syntax highlighting and plugins are behind Cargo features to minimize binary size.

## Common Tasks

### Adding a New Theme
1. Create `themes/mytheme.toml` with color/font settings
2. Add to `BUILTIN_THEMES` in `src/theme/builtin.rs`
3. Implement in `get_builtin_theme()`

### Adding a Keyboard Shortcut
1. Add to `KeybindingsConfig` in `src/config/schema.rs`
2. Handle in `handle_keyboard_shortcuts()` in `src/app/viewer.rs`

### Adding a New Annotation Type
1. Add variant to `AnnotationKind` in `src/annotations/model.rs`
2. Add creation method to `Annotation`
3. Handle rendering in `src/annotations/ui.rs`

### Modifying the Markdown Renderer
The renderer (src/markdown/renderer.rs) processes pulldown-cmark events. Key methods:
- `handle_start_tag()`: Open tags
- `handle_end_tag()`: Close tags and render
- `render_*()`: Specific element rendering

## Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test markdown::

# Run with output
cargo test -- --nocapture
```

## Debugging Tips

1. Enable logging: `RUST_LOG=debug cargo run`
2. Check render cache stats via `state.render_cache.stats()`
3. For egui issues, use `ui.ctx().debug_on_hover()`

## Performance Checklist

- [ ] Use `egui::ScrollArea` with viewport culling
- [ ] Cache expensive computations
- [ ] Avoid allocations in hot paths
- [ ] Profile with `cargo flamegraph`

## Code Style

- Use `rustfmt` with default settings
- Prefer explicit types for public APIs
- Document public items with doc comments
- Keep functions under 50 lines when possible

## Pull Request Guidelines

1. Ensure `cargo test` passes
2. Run `cargo clippy` and address warnings
3. Add tests for new functionality
4. Update documentation as needed
5. Keep commits focused and well-described
