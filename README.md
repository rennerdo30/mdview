# mdview

A desktop Markdown viewer for Linux, macOS and Windows. It renders a file in a native window,
reloads it as soon as it changes on disk, and gets out of the way — no browser, no Electron, no
editing mode.

Written in Rust with [egui](https://github.com/emilk/egui); Markdown is parsed with
[pulldown-cmark](https://github.com/raphlinus/pulldown-cmark).

## Features

- **Native GUI** — single binary desktop app on `eframe`/`egui`
- **Live reload** — the view follows the file while you edit it in another editor
- **Table of contents** — collapsible sidebar with filter, keyboard navigation and the section
  you are reading highlighted
- **Folder browser** — open a directory and switch between its Markdown files
- **Find in document** — incremental search with match count and jump-to-match, plus zoom
- **Annotations** — highlights, notes and bookmarks, stored next to the document
- **PDF export** — from the UI or headless from the command line
- **Dark and light themes** — switchable at runtime, with colour overrides in the config file
- **GitHub-flavoured Markdown** — tables, task lists, footnotes, strikethrough, definition
  lists, math and metadata blocks; `[!NOTE]`-style alert blockquotes can be switched on
- **Syntax highlighting** — code blocks via `syntect`, enabled by default
- **Mermaid diagrams** — rendered through the `mmdc` CLI when it is installed, with a text
  preview as fallback
- **Lua plugins** — optional, behind a feature flag

## Install

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

Optional Cargo features:

| Feature | Default | What it adds |
|---------|---------|--------------|
| `syntax-highlighting` | on | Syntect-based highlighting for fenced code blocks |
| `plugins` | off | Lua plugin runtime (`--features plugins`) |

## Usage

```bash
# Open a file
mdview README.md

# Open a folder and browse its Markdown files
mdview ./docs

# Start in the light theme
mdview --theme light document.md

# Export to PDF and exit
mdview --export-pdf output.pdf document.md

# Hide the table of contents, do not watch the file, custom window size
mdview --no-toc --no-watch --width 1400 --height 900 document.md

# Use a specific config file
mdview --config ./my-config.toml document.md
```

`--reset-file-association` makes mdview ask again about becoming the default `.md` handler.
Run `mdview --help` for the full list of flags.

Recognized Markdown-like extensions are `.md`, `.markdown`, `.mkd`, `.mkdn`, `.mdown`, `.mdwn`,
`.mdtxt`, `.qmd` and `.mdx`. MDX files are opened as Markdown-like text; JSX/ESM blocks are not
rendered as live MDX components.

## Keyboard shortcuts

Use `Cmd` instead of `Ctrl` on macOS. All of these can be remapped in the `[keybindings]`
config section.

| Shortcut | Action |
|----------|--------|
| `Ctrl+O` | Open file |
| `Ctrl+Shift+O` | Open folder |
| `Ctrl+E` | Toggle folder browser |
| `Ctrl+T` | Toggle table of contents |
| `Ctrl+F` | Filter the table of contents |
| `Ctrl+Shift+F` | Find in document (`Enter` / `Shift+Enter` for next / previous match) |
| `Ctrl+H` | Annotate the selection |
| `Ctrl+B` | Add a bookmark |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Zoom in / out / reset (also `Ctrl`+scroll) |
| `F5` | Reload file |
| `Ctrl+P` | Export PDF |
| `Esc` | Close the topmost dialog, find bar or selection |
| `Ctrl+Q` | Quit |

## Configuration

Settings are written to a TOML file in the platform config directory, and the Settings dialog
(`File → Settings…`) edits the same file:

| Platform | Path |
|----------|------|
| Linux | `~/.config/mdview/config.toml` |
| macOS | `~/Library/Application Support/com.mdview.mdview/config.toml` |
| Windows | `%APPDATA%\mdview\mdview\config\config.toml` |

Everything is optional; omitted keys fall back to the defaults.

```toml
[general]
theme = "dark"          # "dark" or "light"
hot_reload = true
show_toc = true
toc_width = 250
check_for_updates = true

[window]
width = 1000
height = 700

[layout]
content_width = 720.0   # reading width in points; omit for full width
content_margin = 48.0
image_width = 600.0

[markdown]
syntax_highlighting = true
show_line_numbers = false
tables = true
task_lists = true
footnotes = true
strikethrough = true
math = true
metadata_blocks = true
definition_lists = true
gfm = false             # true renders [!NOTE]/[!WARNING] alert blockquotes

[theme.fonts]
size = 14.0
line_height = 1.6

[theme.colors]          # override individual colours of the active theme
# background = "#18181e"
# text = "#edeff2"
# link = "#38bdba"
# code_background = "#1e232d"

[annotations]
enabled = true
auto_save = true

[keybindings]
document_search = "Ctrl+Shift+F"

[export]
pdf_theme = "light"
page_size = "A4"        # "A4" or "Letter"
include_toc = true
margin = 20
```

Sections: `general`, `window`, `layout`, `markdown`, `annotations`, `export`, `keybindings`,
`theme`. See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for the full reference.

## Themes

Two themes ship with the app, `dark` and `light`. Pick one in `View → Theme`, in the Settings
dialog, with `--theme`, or via `general.theme` in the config file.

To customise either one, override individual colours, fonts and spacing under `[theme]` in your
config file. The TOML files in `themes/` in this repository are ready-made colour sets to copy
from.

## Annotations

- Select text and right-click (or press `Ctrl+H`) to add a highlight or a note; `Ctrl+B` drops a
  bookmark at the cursor.
- Annotations live in a hidden sidecar file next to the document, named
  `.<filename>.mdview-annotations.json`, so the Markdown file itself is never modified.

## Mermaid diagrams

Mermaid code fences are rendered with the official CLI when it is on `PATH`:

```bash
npm install -g @mermaid-js/mermaid-cli
```

Without `mmdc`, mdview shows a preview block and an install hint instead of failing. Details in
[docs/MERMAID.md](docs/MERMAID.md).

## Plugins

Build with the `plugins` feature and drop Lua scripts into the `plugins/` folder of the config
directory:

```bash
cargo build --release --features plugins
```

The plugin API is documented in [docs/PLUGIN_SDK.md](docs/PLUGIN_SDK.md).

## Development

```bash
cargo test                                                  # test suite
cargo test --test performance_regression -- --ignored       # performance checks
cargo clippy --all-targets                                  # lints
```

More docs: [ARCHITECTURE.md](docs/ARCHITECTURE.md), [DEVELOPER_GUIDE.md](docs/DEVELOPER_GUIDE.md),
[USER_GUIDE.md](docs/USER_GUIDE.md), [CONTRIBUTING.md](CONTRIBUTING.md).

## Tech stack

Rust, `eframe`/`egui` (GUI), `pulldown-cmark` (parsing), `syntect` (highlighting), `notify`
(file watching), `printpdf` (PDF export), `clap` (CLI), `mlua` (optional plugins).

## License

MIT — see [LICENSE](LICENSE).
