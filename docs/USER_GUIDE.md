# mdview - User Guide

Welcome to mdview, a fast and extensible markdown viewer. This guide covers installation, basic usage, and all features.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [User Interface](#user-interface)
4. [Keyboard Shortcuts](#keyboard-shortcuts)
5. [Features](#features)
6. [Configuration](#configuration)
7. [Themes](#themes)
8. [Troubleshooting](#troubleshooting)

---

## Installation

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/yourusername/mdview/releases):

| Platform | File |
|----------|------|
| Linux (x86_64) | `mdview-linux-x86_64` |
| macOS (Intel) | `mdview-macos-x86_64` |
| macOS (Apple Silicon) | `mdview-macos-aarch64` |
| Windows | `mdview-windows-x86_64.exe` |

**Linux/macOS:**
```bash
chmod +x mdview-*
sudo mv mdview-* /usr/local/bin/mdview
```

**Windows:**
Move `mdview-windows-x86_64.exe` to a directory in your PATH.

### From Source

```bash
# Clone repository
git clone https://github.com/yourusername/mdview.git
cd mdview

# Build release
cargo build --release

# Install
cargo install --path .
```

### Cargo Install

```bash
cargo install mdview
```

---

## Quick Start

### Open a File

```bash
# From command line
mdview README.md

# With specific theme
mdview --theme light document.md
```

### Drag and Drop

Simply drag a markdown file onto the mdview window.

### File Menu

Use **File → Open** or press `Ctrl+O` to open a file.

mdview recognizes common Markdown-like extensions including `.md`, `.markdown`,
`.mkd`, `.mkdn`, `.mdown`, `.mdwn`, `.mdtxt`, `.qmd`, and `.mdx`. MDX files are
displayed as Markdown-like documents; JSX/ESM content is not rendered as live MDX
components.

---

## User Interface

```
┌─────────────────────────────────────────────────────────────┐
│  File   View   Help                              [Menu Bar] │
├─────────────┬───────────────────────────────────────────────┤
│             │                                               │
│   Contents  │                                               │
│   ────────  │            Markdown Content                   │
│   • Heading │                                               │
│     • Sub   │            Rendered here                      │
│   • Heading │                                               │
│             │                                               │
│  [TOC Panel]│                            [Main Content]     │
├─────────────┴───────────────────────────────────────────────┤
│  /path/to/file.md                            Watching       │
│                                              [Status Bar]   │
└─────────────────────────────────────────────────────────────┘
```

### Components

| Component | Description |
|-----------|-------------|
| Menu Bar | File, View, Help menus |
| TOC Panel | Collapsible table of contents (left) |
| Main Content | Rendered markdown (center) |
| Status Bar | File path, status messages |

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+O` | Open file |
| `Ctrl+T` | Toggle TOC sidebar |
| `Ctrl+P` | Export to PDF |
| `F5` | Reload file |
| `Ctrl+R` | Reload file (alternative) |
| `Ctrl+Q` | Quit |
| `Escape` | Cancel annotation |

---

## Features

### Table of Contents

The TOC sidebar shows all headings (H1-H6) in your document.

- **Toggle**: Press `Ctrl+T` or use View menu
- **Navigate**: Click any heading to scroll
- **Collapse**: Click arrows to collapse/expand sections
- **Highlight**: Current section is highlighted

### Hot Reload

When you edit the file externally, mdview automatically reloads:

- Changes detected within 100ms
- Scroll position preserved
- Status bar shows "File reloaded"

To disable: `mdview --no-watch file.md`

### PDF Export

Export your rendered markdown to PDF:

1. Press `Ctrl+P` or use **File → Export PDF**
2. PDF saved as `filename.pdf` (same directory)
3. Status bar confirms export

**CLI Export:**
```bash
mdview --export-pdf output.pdf document.md
```

### Annotations

Add highlights, notes, and bookmarks to your documents.

#### Creating Annotations

1. Select text in the document
2. Right-click to open annotation menu
3. Choose: Highlight, Note, or Bookmark

#### Highlight Colors

- Yellow (default)
- Green
- Blue
- Red
- Purple
- Orange

#### Notes

- Click margin icon to view/edit
- Notes persist across sessions

#### Storage

Annotations saved as `.filename.mdview-annotations.json` next to your file.

---

## Configuration

### Config Location

| Platform | Path |
|----------|------|
| Linux | `~/.config/mdview/config.toml` |
| macOS | `~/Library/Application Support/com.mdview.mdview/config.toml` |
| Windows | `%APPDATA%\mdview\mdview\config.toml` |

### Example Config

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
default_highlight_color = "#ffeb3b"

[export]
pdf_theme = "light"
include_toc = true
page_size = "A4"
margin = 20

[keybindings]
toggle_toc = "Ctrl+T"
export_pdf = "Ctrl+P"
reload = "F5"
```

See [CONFIGURATION.md](CONFIGURATION.md) for all options.

---

## Themes

### Built-in Themes

| Theme | Description |
|-------|-------------|
| `dark` | Dark background, light text (default) |
| `light` | Light background, dark text |
| `sepia` | Warm, paper-like tones |
| `high-contrast` | Maximum contrast for accessibility |

### Changing Theme

**Command line:**
```bash
mdview --theme light document.md
```

**Config file:**
```toml
[general]
theme = "sepia"
```

### Custom Themes

Create `~/.config/mdview/themes/mytheme.toml`:

```toml
[colors]
background = "#1e1e1e"
text = "#d4d4d4"
heading = "#569cd6"
link = "#4ec9b0"
code_background = "#2d2d2d"
code_text = "#ce9178"

[fonts]
size = 14.0
line_height = 1.5

[spacing]
paragraph = 12.0
heading_top = 24.0
```

Then use: `mdview --theme mytheme document.md`

---

## Troubleshooting

### Common Issues

#### Blank Window

- Check file exists: `ls -la file.md`
- Check file encoding (must be UTF-8)
- Enable logging: `RUST_LOG=debug mdview file.md`

#### Theme Not Applied

- Verify TOML syntax: `cat config.toml`
- Check theme name spelling
- Try built-in theme: `--theme dark`

#### Hot Reload Not Working

- Check `hot_reload = true` in config
- Verify file permissions
- Check `--no-watch` flag not set

#### PDF Export Fails

- Check write permissions in directory
- Verify enough disk space
- Check status bar for error message

### Getting Help

1. Check [GitHub Issues](https://github.com/yourusername/mdview/issues)
2. Enable debug logging: `RUST_LOG=debug`
3. Open a new issue with:
   - mdview version
   - Operating system
   - Steps to reproduce
   - Error messages

---

## Tips & Tricks

### Open from Editor

Configure your editor to open markdown files in mdview:

**VS Code** (`settings.json`):
```json
{
  "markdown.preview.externalViewer": "mdview"
}
```

**Vim** (`.vimrc`):
```vim
autocmd FileType markdown nnoremap <leader>p :!mdview %<CR>
```

### Create Alias

```bash
# ~/.bashrc or ~/.zshrc
alias md='mdview'
```

### Quick PDF

```bash
# Export and open
mdview --export-pdf out.pdf doc.md && open out.pdf
```

---

Enjoy using mdview!
