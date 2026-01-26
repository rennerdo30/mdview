-- Example mdview plugin
-- This plugin demonstrates the basic plugin API

-- Plugin metadata
local plugin = {
    name = "Hello Plugin",
    version = "1.0.0",
    author = "mdview",
    description = "A simple example plugin"
}

-- Called when a file is opened
mdview_hooks.on_file_open = function(filepath)
    mdview.log_info("File opened: " .. filepath)
end

-- Called when a file is closed
mdview_hooks.on_file_close = function()
    mdview.log_info("File closed")
end

-- Called before rendering
mdview_hooks.on_pre_render = function()
    -- Can modify render settings here
end

-- Called after rendering
mdview_hooks.on_post_render = function()
    -- Can add overlays or post-processing here
end

-- Called when theme changes
mdview_hooks.on_theme_change = function(theme_name)
    mdview.log_info("Theme changed to: " .. theme_name)
end

-- Called when annotation is added
mdview_hooks.on_annotation_add = function(annotation_id, kind)
    mdview.log_info("Annotation added: " .. annotation_id .. " (" .. kind .. ")")
end

-- Called when annotation is removed
mdview_hooks.on_annotation_remove = function(annotation_id)
    mdview.log_info("Annotation removed: " .. annotation_id)
end

-- Log that plugin loaded successfully
mdview.log_info("Hello plugin loaded!")
