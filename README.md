# mdview

A desktop Markdown viewer for Linux, macOS and Windows. It renders a file in a native window,
reloads it as soon as it changes on disk, and gets out of the way — no browser, no Electron, no
editing mode.

Written in Rust with [egui](https://github.com/emilk/egui); Markdown is parsed with
[pulldown-cmark](https://github.com/raphlinus/pulldown-cmark).

## Features

- **Live reload** — the view follows the file while you edit it in another editor
- **Table of contents** — collapsible sidebar with filter, keyboard navigation and the section
  you are reading highlighted
- **Folder browser** — open a directory and switch between its Markdown files
- **Find in document** — incremental search with match count and jump-to-match
- **Annotations** — highlights, notes and bookmarks, stored next to the document
- **PDF export** — from the UI or headless from the command line
- **Dark and light themes** — switchable at runtime, with colour overrides in the config file
- **GitHub-flavoured Markdown** — tables, task lists, footnotes, strikethrough, definition
  lists, metadata blocks and fenced code with syntax highlighting; `[!NOTE]`-style alert
  blockquotes can be switched on
- **Mermaid diagrams** — rendered through the Mermaid CLI when it is installed, with a text
  preview as fallback (see [docs/MERMAID.md](docs/MERMAID.md))
- **Lua plugins** — optional, behind a feature flag

## Install

There is no published binary release yet, so build it from source. You need a
[Rust toolchain](https://rustup.rs).

```bash
git clone https://github.com/rennerdo30/mdview
cd mdview
cargo build --release
# binary at target/release/mdview
```

Or install it straight into `~/.cargo/bin`:

```bash
cargo install --git https://github.com/rennerdo30/mdview
```

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

# Open without the table of contents, and without watching the file
mdview --no-toc --no-watch document.md
```

Further flags: `--width` / `--height` for the initial window size, `--config <FILE>` for an
alternate config file, `--reset-file-association` to be asked again about becoming the default
`.md` handler. `mdview --help` lists all of them.

Recognized Markdown-like extensions are `.md`, `.markdown`, `.mkd`, `.mkdn`, `.mdown`, `.mdwn`,
`.mdtxt`, `.qmd` and `.mdx`. MDX files are opened as Markdown-like text; JSX/ESM blocks are not
rendered as live MDX components.

## Keyboard shortcuts

Use `Cmd` instead of `Ctrl` on macOS. All of these can be remapped in the config file.

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

[theme.colors]        # override individual colours of the active theme
# background = "#18181e"
# text = "#edeff2"
# link = "#38bdba"
# code_background = "#1e232d"

[keybindings]
document_search = "Ctrl+Shift+F"

[export]
pdf_theme = "light"
page_size = "A4"        # "A4" or "Letter"
include_toc = true
margin = 20
```

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for the full schema.

## Annotations

- Select text and right-click (or press `Ctrl+H`) to add a highlight or a note; `Ctrl+B` drops a
  bookmark at the cursor.
- Annotations live in a hidden sidecar file next to the document, named
  `.<filename>.mdview-annotations.json`, so the Markdown file itself is never modified.

## Plugins

Build with the `plugins` feature and drop Lua scripts into the `plugins/` folder of the config
directory:

```bash
cargo build --release --features plugins
```

The plugin API is documented in [docs/PLUGIN_SDK.md](docs/PLUGIN_SDK.md).

## Documentation

- [docs/USER_GUIDE.md](docs/USER_GUIDE.md) — day-to-day usage
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) — every config key
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the app is put together
- [docs/DEVELOPER_GUIDE.md](docs/DEVELOPER_GUIDE.md) — building, testing, contributing
- [CONTRIBUTING.md](CONTRIBUTING.md) — pull request checklist

## License

MIT — see [LICENSE](LICENSE).
