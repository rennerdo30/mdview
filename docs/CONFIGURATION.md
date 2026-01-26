# mdview - Configuration Reference

Complete reference for all configuration options in mdview.

## Table of Contents

1. [Configuration File Location](#configuration-file-location)
2. [General Settings](#general-settings)
3. [Window Settings](#window-settings)
4. [Markdown Settings](#markdown-settings)
5. [Annotations Settings](#annotations-settings)
6. [Export Settings](#export-settings)
7. [Keybindings](#keybindings)
8. [Theme Customization](#theme-customization)
9. [Complete Example](#complete-example)

---

## Configuration File Location

| Platform | Path |
|----------|------|
| Linux | `~/.config/mdview/config.toml` |
| macOS | `~/Library/Application Support/com.mdview.mdview/config.toml` |
| Windows | `%APPDATA%\mdview\mdview\config.toml` |

Create the directory if it doesn't exist:

```bash
# Linux/macOS
mkdir -p ~/.config/mdview

# Windows (PowerShell)
New-Item -ItemType Directory -Force -Path "$env:APPDATA\mdview\mdview"
```

---

## General Settings

```toml
[general]
theme = "dark"          # Theme name: "dark", "light", "sepia", "high-contrast", or custom
hot_reload = true       # Watch file for changes and auto-reload
show_toc = true         # Show table of contents sidebar by default
toc_width = 250         # Default TOC sidebar width in pixels
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"dark"` | Theme to use |
| `hot_reload` | bool | `true` | Enable file watching |
| `show_toc` | bool | `true` | Show TOC on startup |
| `toc_width` | u32 | `250` | TOC width in pixels |

---

## Window Settings

```toml
[window]
width = 1000            # Initial window width
height = 700            # Initial window height
maximized = false       # Start maximized
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `width` | u32 | `1000` | Window width in pixels |
| `height` | u32 | `700` | Window height in pixels |
| `maximized` | bool | `false` | Start maximized |

---

## Markdown Settings

```toml
[markdown]
tables = true                   # Enable GitHub-style tables
strikethrough = true            # Enable ~~strikethrough~~
task_lists = true               # Enable - [ ] task lists
footnotes = true                # Enable [^1] footnotes
smart_punctuation = false       # Convert quotes, dashes
syntax_highlighting = true      # Highlight code blocks
syntax_theme = "base16-ocean.dark"  # Syntect theme name
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `tables` | bool | `true` | GFM tables |
| `strikethrough` | bool | `true` | ~~text~~ support |
| `task_lists` | bool | `true` | Checkbox lists |
| `footnotes` | bool | `true` | Footnote references |
| `smart_punctuation` | bool | `false` | Smart quotes/dashes |
| `syntax_highlighting` | bool | `true` | Code highlighting |
| `syntax_theme` | string | `"base16-ocean.dark"` | Highlight theme |

### Available Syntax Themes

- `base16-ocean.dark`
- `base16-ocean.light`
- `base16-eighties.dark`
- `base16-mocha.dark`
- `InspiredGitHub`
- `Solarized (dark)`
- `Solarized (light)`

---

## Annotations Settings

```toml
[annotations]
enabled = true                          # Enable annotation system
auto_save = true                        # Auto-save on change
default_highlight_color = "#ffeb3b"     # Default highlight color (hex)
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `true` | Enable annotations |
| `auto_save` | bool | `true` | Save automatically |
| `default_highlight_color` | string | `"#ffeb3b"` | Default color |

### Highlight Colors

Predefined colors available in the UI:

| Color | Hex |
|-------|-----|
| Yellow | `#ffeb3b` |
| Green | `#4caf50` |
| Blue | `#2196f3` |
| Red | `#f44336` |
| Purple | `#9c27b0` |
| Orange | `#ff9800` |

---

## Export Settings

```toml
[export]
pdf_theme = "light"     # Theme for PDF export
include_toc = true      # Include TOC in PDF
page_size = "A4"        # Page size: "A4" or "Letter"
margin = 20             # Margin in mm
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `pdf_theme` | string | `"light"` | PDF theme |
| `include_toc` | bool | `true` | TOC in PDF |
| `page_size` | string | `"A4"` | Page size |
| `margin` | u32 | `20` | Margin in mm |

### Page Sizes

| Size | Dimensions |
|------|------------|
| A4 | 210mm × 297mm |
| Letter | 215.9mm × 279.4mm |

---

## Keybindings

```toml
[keybindings]
toggle_toc = "Ctrl+T"   # Toggle TOC sidebar
export_pdf = "Ctrl+P"   # Export to PDF
reload = "F5"           # Reload file
open_file = "Ctrl+O"    # Open file dialog
quit = "Ctrl+Q"         # Quit application
```

| Option | Default | Description |
|--------|---------|-------------|
| `toggle_toc` | `"Ctrl+T"` | Toggle TOC |
| `export_pdf` | `"Ctrl+P"` | Export PDF |
| `reload` | `"F5"` | Reload file |
| `open_file` | `"Ctrl+O"` | Open file |
| `quit` | `"Ctrl+Q"` | Quit |

### Keybinding Format

- Modifiers: `Ctrl`, `Alt`, `Shift`, `Super`
- Keys: `A`-`Z`, `0`-`9`, `F1`-`F12`, etc.
- Combine with `+`: `Ctrl+Shift+S`

---

## Theme Customization

### Colors

```toml
[theme.colors]
background = "#1e1e1e"           # Main background
text = "#d4d4d4"                 # Body text
heading = "#569cd6"              # Heading text
link = "#4ec9b0"                 # Link text
code_background = "#2d2d2d"      # Code block background
code_text = "#ce9178"            # Code text
sidebar_background = "#252526"   # TOC sidebar background
selection = "#264f78"            # Selection highlight
```

All colors use hex format: `#RRGGBB`

### Fonts

```toml
[theme.fonts]
body = "sans-serif"      # Body font family
heading = "sans-serif"   # Heading font family
code = "monospace"       # Code font family
size = 14.0              # Base font size
line_height = 1.5        # Line height multiplier
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `body` | string | `"sans-serif"` | Body font |
| `heading` | string | `"sans-serif"` | Heading font |
| `code` | string | `"monospace"` | Code font |
| `size` | f32 | `14.0` | Base size in points |
| `line_height` | f32 | `1.5` | Line height multiplier |

### Spacing

```toml
[theme.spacing]
paragraph = 12.0         # Space after paragraphs
heading_top = 24.0       # Space before headings
heading_bottom = 8.0     # Space after headings
list_indent = 20.0       # List item indent
code_padding = 8.0       # Code block padding
```

All spacing values in pixels.

---

## Complete Example

```toml
# mdview configuration

[general]
theme = "dark"
hot_reload = true
show_toc = true
toc_width = 280

[window]
width = 1200
height = 800
maximized = false

[markdown]
tables = true
strikethrough = true
task_lists = true
footnotes = true
smart_punctuation = false
syntax_highlighting = true
syntax_theme = "base16-ocean.dark"

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
open_file = "Ctrl+O"
quit = "Ctrl+Q"

[theme.colors]
background = "#1e1e1e"
text = "#d4d4d4"
heading = "#569cd6"
link = "#4ec9b0"
code_background = "#2d2d2d"
code_text = "#ce9178"
sidebar_background = "#252526"
selection = "#264f78"

[theme.fonts]
body = "sans-serif"
heading = "sans-serif"
code = "monospace"
size = 14.0
line_height = 1.5

[theme.spacing]
paragraph = 12.0
heading_top = 24.0
heading_bottom = 8.0
list_indent = 20.0
code_padding = 8.0
```

---

## CLI Overrides

Configuration can be overridden via command line:

```bash
# Override theme
mdview --theme light file.md

# Disable hot reload
mdview --no-watch file.md

# Hide TOC
mdview --no-toc file.md

# Custom window size
mdview --width 1400 --height 900 file.md

# Use custom config file
mdview --config /path/to/config.toml file.md
```

CLI arguments take precedence over config file settings.
