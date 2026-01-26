# mdview - Plugin SDK

Guide for developing Lua plugins for mdview.

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [Plugin API](#plugin-api)
4. [Hooks](#hooks)
5. [Examples](#examples)
6. [Best Practices](#best-practices)
7. [Limitations](#limitations)

---

## Overview

mdview supports Lua 5.4 plugins for extending functionality. Plugins run in a sandboxed environment with access to the mdview API.

### Requirements

- Build mdview with `--features plugins`
- Lua plugins in `~/.config/mdview/plugins/`

### Security

The Lua environment is sandboxed:
- No file I/O (`io`, `dofile`, `loadfile`)
- No OS access (`os`)
- No debug library
- Safe functions: `string`, `table`, `math`, `pairs`, `ipairs`, etc.

---

## Getting Started

### Enable Plugin Support

```bash
# Build with plugins feature
cargo build --release --features plugins
```

### Create a Plugin

Create `~/.config/mdview/plugins/myplugin.lua`:

```lua
-- My first mdview plugin

-- Log when file opens
mdview_hooks.on_file_open = function(filepath)
    mdview.log_info("Opened: " .. filepath)
end

mdview.log_info("My plugin loaded!")
```

### Plugin Loading

Plugins are loaded automatically from:
- `~/.config/mdview/plugins/*.lua`

---

## Plugin API

### Global Objects

#### `mdview`

Main API namespace.

```lua
-- Version info
local version = mdview.version  -- e.g., "0.1.0"

-- Logging
mdview.log_info("Info message")
mdview.log_warn("Warning message")
mdview.log_error("Error message")
```

#### `mdview_hooks`

Table for registering event hooks.

```lua
mdview_hooks.on_file_open = function(filepath)
    -- Called when file is opened
end
```

### API Reference

| Function | Parameters | Description |
|----------|------------|-------------|
| `mdview.version` | - | Get mdview version string |
| `mdview.log_info(msg)` | string | Log info message |
| `mdview.log_warn(msg)` | string | Log warning message |
| `mdview.log_error(msg)` | string | Log error message |

---

## Hooks

Register hooks by assigning functions to `mdview_hooks`:

### on_file_open

Called when a file is opened.

```lua
mdview_hooks.on_file_open = function(filepath)
    -- filepath: string - absolute path to file
    mdview.log_info("File opened: " .. filepath)
end
```

### on_file_close

Called when a file is closed.

```lua
mdview_hooks.on_file_close = function()
    mdview.log_info("File closed")
end
```

### on_pre_render

Called before rendering markdown.

```lua
mdview_hooks.on_pre_render = function()
    -- Modify render settings
end
```

### on_post_render

Called after rendering markdown.

```lua
mdview_hooks.on_post_render = function()
    -- Add overlays or effects
end
```

### on_theme_change

Called when theme changes.

```lua
mdview_hooks.on_theme_change = function(theme_name)
    -- theme_name: string - new theme name
    mdview.log_info("Theme: " .. theme_name)
end
```

### on_annotation_add

Called when annotation is added.

```lua
mdview_hooks.on_annotation_add = function(id, kind)
    -- id: string - annotation ID
    -- kind: string - "highlight", "note", or "bookmark"
    mdview.log_info("Added " .. kind .. ": " .. id)
end
```

### on_annotation_remove

Called when annotation is removed.

```lua
mdview_hooks.on_annotation_remove = function(id)
    -- id: string - annotation ID
    mdview.log_info("Removed: " .. id)
end
```

### Hook Reference

| Hook | Parameters | When Called |
|------|------------|-------------|
| `on_file_open` | filepath | File opened |
| `on_file_close` | - | File closed |
| `on_pre_render` | - | Before render |
| `on_post_render` | - | After render |
| `on_theme_change` | theme_name | Theme changed |
| `on_annotation_add` | id, kind | Annotation created |
| `on_annotation_remove` | id | Annotation deleted |

---

## Examples

### Basic Plugin

```lua
-- basic.lua
-- A simple example plugin

local plugin = {
    name = "Basic Plugin",
    version = "1.0.0"
}

mdview_hooks.on_file_open = function(filepath)
    mdview.log_info(plugin.name .. ": Opened " .. filepath)
end

mdview.log_info(plugin.name .. " v" .. plugin.version .. " loaded")
```

### Statistics Plugin

```lua
-- stats.lua
-- Track usage statistics

local stats = {
    files_opened = 0,
    annotations_added = 0,
    theme_changes = 0
}

mdview_hooks.on_file_open = function(filepath)
    stats.files_opened = stats.files_opened + 1
    mdview.log_info("Files opened this session: " .. stats.files_opened)
end

mdview_hooks.on_annotation_add = function(id, kind)
    stats.annotations_added = stats.annotations_added + 1
end

mdview_hooks.on_theme_change = function(theme_name)
    stats.theme_changes = stats.theme_changes + 1
end

mdview.log_info("Stats plugin loaded")
```

### Filename Logger

```lua
-- filename_logger.lua
-- Log all opened filenames

local log = {}

mdview_hooks.on_file_open = function(filepath)
    -- Extract filename from path
    local filename = filepath:match("([^/\\]+)$") or filepath
    table.insert(log, {
        filename = filename,
        time = os.time and os.time() or 0
    })
    mdview.log_info("Logged: " .. filename)
end

mdview_hooks.on_file_close = function()
    mdview.log_info("Total files logged: " .. #log)
end

mdview.log_info("Filename logger ready")
```

### Theme Announcer

```lua
-- theme_announcer.lua
-- Announce theme changes

local theme_descriptions = {
    dark = "Easy on the eyes for night coding",
    light = "Crisp and clean for daytime",
    sepia = "Warm and comfortable for reading",
    ["high-contrast"] = "Maximum visibility"
}

mdview_hooks.on_theme_change = function(theme_name)
    local desc = theme_descriptions[theme_name] or "Custom theme"
    mdview.log_info("Theme: " .. theme_name .. " - " .. desc)
end

mdview.log_info("Theme announcer active")
```

---

## Best Practices

### 1. Use Descriptive Names

```lua
-- Good
mdview_hooks.on_file_open = function(filepath)

-- Avoid
mdview_hooks.on_file_open = function(x)
```

### 2. Handle Errors Gracefully

```lua
mdview_hooks.on_file_open = function(filepath)
    if not filepath then
        mdview.log_warn("No filepath provided")
        return
    end
    -- Process filepath
end
```

### 3. Log Appropriately

```lua
-- Use correct log levels
mdview.log_info("Normal operation")
mdview.log_warn("Potential issue")
mdview.log_error("Critical problem")
```

### 4. Keep Hooks Fast

```lua
-- Avoid heavy computation in hooks
mdview_hooks.on_pre_render = function()
    -- Quick operations only
    -- Heavy work affects frame rate
end
```

### 5. Document Your Plugin

```lua
--[[
    My Plugin v1.0.0

    Description: What it does
    Author: Your name
    License: MIT

    Usage:
    - Place in ~/.config/mdview/plugins/
    - Restart mdview
]]
```

---

## Limitations

### Sandboxed Environment

Not available:
- `io` - File operations
- `os` - System access
- `debug` - Debug library
- `dofile`, `loadfile` - File loading

### No UI Modification

Plugins cannot:
- Add UI elements
- Modify rendering directly
- Access egui context

### Synchronous Execution

- Hooks run synchronously
- Long operations block rendering
- Keep hooks fast (<1ms)

### No Network

- No HTTP requests
- No socket access
- No external communication

---

## Future API

Planned additions (not yet implemented):

```lua
-- File content access (planned)
local content = mdview.get_content()

-- Annotation management (planned)
mdview.add_highlight(start, end, color)
mdview.add_note(start, end, text)

-- UI notifications (planned)
mdview.notify("Message", "info")

-- Settings access (planned)
local theme = mdview.get_setting("theme")
mdview.set_setting("show_toc", true)
```

---

## Troubleshooting

### Plugin Not Loading

1. Check file location: `~/.config/mdview/plugins/`
2. Verify `.lua` extension
3. Check for syntax errors
4. Enable debug logging: `RUST_LOG=debug`

### Hook Not Called

1. Verify hook name spelling
2. Check function signature
3. Ensure plugin loaded (check logs)

### Errors in Plugin

Check logs for Lua errors:
```bash
RUST_LOG=mdview=debug mdview file.md
```

---

## Support

- [GitHub Issues](https://github.com/yourusername/mdview/issues)
- [Plugin Examples](https://github.com/yourusername/mdview/tree/main/plugins/examples)
