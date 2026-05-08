# mdview

A fast, cross-platform markdown viewer built with Rust and egui.

## Features

- **Fast**: < 50ms cold start, smooth 60fps rendering
- **Hot Reload**: Automatically refreshes when file changes
- **Table of Contents**: Collapsible sidebar with clickable navigation
- **Annotations**: Highlight text, add notes, create bookmarks
- **PDF Export**: Export rendered markdown to PDF
- **Themes**: Dark, light, sepia, and custom TOML themes
- **Plugins**: Optional Lua scripting support

## Installation

```bash
cargo install mdview
```

Or build from source:

```bash
git clone https://github.com/rennerdo30/mdview
cd mdview
cargo build --release
```

## Usage

```bash
# Open a markdown file
mdview README.md

# With specific theme
mdview --theme light document.md

# Export to PDF
mdview --export-pdf output.pdf document.md

# Disable hot reload
mdview --no-watch document.md
```

Recognized Markdown-like extensions include `.md`, `.markdown`, `.mkd`, `.mkdn`,
`.mdown`, `.mdwn`, `.mdtxt`, `.qmd`, and `.mdx`. MDX files are opened as
Markdown-like text; JSX/ESM blocks are not rendered as live MDX components.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+O` | Open file |
| `Ctrl+T` | Toggle TOC |
| `Ctrl+P` | Export PDF |
| `F5` | Reload file |
| `Ctrl+Q` | Quit |

## Configuration

Create `~/.config/mdview/config.toml`:

```toml
[general]
theme = "dark"
hot_reload = true
show_toc = true

[markdown]
syntax_highlighting = true
tables = true

[export]
pdf_theme = "light"
page_size = "A4"
```

## Annotations

- Select text → Right-click → Add highlight or note
- Click margin icons to view/edit annotations
- Annotations saved as `.mdview-annotations.json` sidecar files

## Themes

Built-in themes: `dark`, `light`, `sepia`, `high-contrast`

Custom themes: Create TOML files in `~/.config/mdview/themes/`

## Plugins

Enable with `--features plugins`:

```bash
cargo build --release --features plugins
```

Place Lua scripts in `~/.config/mdview/plugins/`

## License

MIT License
