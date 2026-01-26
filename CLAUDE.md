# mdview Development Guide

## Quick Reference

**Stack:** Rust + egui/eframe + pulldown-cmark + muda (native menus)

**Build:** `cargo build` | **Test:** `cargo test` | **Run:** `cargo run -- file.md`

**Features:** `--features "mermaid,plugins,syntax-highlighting"`

## Directory Structure

```
src/
├── main.rs           # Entry, CLI, native menu init
├── app/viewer.rs     # Main app, UI, event handling
├── app/state.rs      # AppState - all mutable state
├── markdown/         # Parser + renderer (pulldown-cmark → egui)
├── native_menu.rs    # macOS/Win/Linux native menu bar (muda)
├── config/           # TOML config schema + loading
├── theme/            # Styling, colors, fonts
├── export/pdf.rs     # PDF export (printpdf)
├── annotations/      # Highlights, notes, bookmarks
├── toc/              # Table of contents sidebar
├── watcher/          # File change detection
└── plugin/           # Lua plugins (feature-gated)
```

## Key Patterns

- **State:** All in `AppState` (src/app/state.rs)
- **Rendering:** pulldown-cmark events → egui widgets (no AST)
- **Async:** Mermaid renders in background threads via channels
- **Menus:** Native via muda crate, in-window via egui

## Common Edits

| Task | Files |
|------|-------|
| Add keyboard shortcut | `config/schema.rs`, `app/viewer.rs:handle_keyboard_shortcuts` |
| Add menu item | `native_menu.rs`, `app/viewer.rs:handle_native_menu_events` |
| Modify markdown render | `markdown/renderer.rs:handle_end_tag`, `render_*` methods |
| Add theme | `theme/builtin.rs` |
| Add annotation type | `annotations/model.rs`, `annotations/ui.rs` |

## Testing

```bash
cargo test                    # All tests
cargo test markdown::         # Module tests
RUST_LOG=debug cargo run     # Debug logging
```

## Checklist Before Commit

- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] `cargo check` no warnings
- [ ] Update TODO.md if needed
