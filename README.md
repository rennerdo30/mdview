# mdview

A fast, cross-platform markdown viewer built with Rust and egui.

## Features

- **Native GUI**: Single binary desktop app (Windows, macOS, Linux) built on `eframe`/`egui`
- **Hot Reload**: Automatically refreshes when the open file changes
- **Table of Contents**: Collapsible sidebar with clickable navigation and TOC search
- **File Browser**: Open a folder and browse its markdown files in a sidebar
- **Search & Zoom**: In-document search, zoom in/out/reset
- **Annotations**: Highlight text, add notes, create bookmarks
- **PDF Export**: Export rendered markdown to PDF
- **Themes**: Built-in dark, light, sepia and high-contrast themes plus custom TOML themes
- **Markdown extras**: Tables, task lists, footnotes, definition lists, math, metadata blocks
- **Syntax Highlighting**: Code blocks via `syntect` (enabled by default)
- **Mermaid**: Diagrams rendered through the `mmdc` CLI when it is installed
- **Plugins**: Optional Lua scripting support

## Installation

Prebuilt binaries for Linux, macOS (x86_64 / aarch64) and Windows are attached to the
[`nightly` release](https://github.com/rennerdo30/mdview/releases/tag/nightly).

Build from source (stable Rust toolchain, edition 2021):

```bash
git clone https://github.com/rennerdo30/mdview
cd mdview
cargo build --release
# binary at target/release/mdview
```

Or install it straight from the repository:

```bash
cargo install --git https://github.com/rennerdo30/mdview
```

> Note: the `mdview` name on crates.io belongs to an unrelated crate, so `cargo install mdview`
> does **not** install this project.

## Usage

```bash
# Open a markdown file
mdview README.md

# Open a folder in the file browser
mdview ./docs

# With a specific theme
mdview --theme light document.md

# Export to PDF and exit
mdview --export-pdf output.pdf document.md

# Disable hot reload
mdview --no-watch document.md

# Start with the TOC sidebar hidden, custom window size
mdview --no-toc --width 1400 --height 900 document.md

# Use a specific config file
mdview --config ./my-config.toml document.md
```

Run `mdview --help` for the full list of flags.

Recognized Markdown-like extensions include `.md`, `.markdown`, `.mkd`, `.mkdn`,
`.mdown`, `.mdwn`, `.mdtxt`, `.qmd`, and `.mdx`. MDX files are opened as
Markdown-like text; JSX/ESM blocks are not rendered as live MDX components.

## Keyboard Shortcuts

Defaults (all remappable via the `[keybindings]` config section):

| Shortcut | Action |
|----------|--------|
| `Ctrl+O` | Open file |
| `Ctrl+Shift+O` | Open folder |
| `Ctrl+E` | Toggle file browser |
| `Ctrl+T` | Toggle TOC |
| `Ctrl+F` | Focus TOC search |
| `Ctrl+Shift+F` | Find in document |
| `Ctrl+H` | Add annotation |
| `Ctrl+B` | Add bookmark |
| `Ctrl+P` | Export PDF |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Zoom in / out / reset |
| `F5` | Reload file |
| `Ctrl+Q` | Quit |

## Configuration

Create `~/.config/mdview/config.toml`:

```toml
[general]
theme = "dark"
hot_reload = true
show_toc = true
toc_width = 250
check_for_updates = true

[window]
width = 1000
height = 700

[markdown]
syntax_highlighting = true
tables = true
math = true
metadata_blocks = true
definition_lists = true

[annotations]
enabled = true
auto_save = true

[export]
pdf_theme = "light"
page_size = "A4"
```

Sections available: `general`, `window`, `markdown`, `annotations`, `export`, `keybindings`,
`theme`, `layout`. See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for the full reference.

## Annotations

- Select text → Right-click → Add highlight or note
- Click margin icons to view/edit annotations
- Annotations saved as `.mdview-annotations.json` sidecar files

## Themes

Built-in themes: `dark`, `light`, `sepia`, `high-contrast`

Custom themes: Create TOML files in `~/.config/mdview/themes/` (see `themes/` in this repo
for examples), or override individual colors and fonts in the `[theme]` config section.

## Mermaid Diagrams

Mermaid code fences are rendered with the official CLI when it is on `PATH`:

```bash
npm install -g @mermaid-js/mermaid-cli
```

Without `mmdc`, mdview shows a fallback block and an install hint instead of failing.
Details in [docs/MERMAID.md](docs/MERMAID.md).

## Plugins

Enable with `--features plugins`:

```bash
cargo build --release --features plugins
```

Place Lua scripts in `~/.config/mdview/plugins/`. See [docs/PLUGIN_SDK.md](docs/PLUGIN_SDK.md).

## Development

```bash
cargo test                                                  # test suite
cargo test --test performance_regression -- --ignored       # performance checks
```

More docs: [ARCHITECTURE.md](docs/ARCHITECTURE.md), [DEVELOPER_GUIDE.md](docs/DEVELOPER_GUIDE.md),
[USER_GUIDE.md](docs/USER_GUIDE.md), [CONTRIBUTING.md](CONTRIBUTING.md).

## Tech Stack

Rust, `eframe`/`egui` (GUI), `pulldown-cmark` (parsing), `syntect` (highlighting),
`notify` (file watching), `printpdf` (PDF export), `clap` (CLI), `mlua` (optional plugins).

## License

MIT License — see [LICENSE](LICENSE).
