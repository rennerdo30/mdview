//! Plugin API for Lua scripts

#[cfg(feature = "plugins")]
use mlua::{Lua, Result as LuaResult, Value};

#[cfg(feature = "plugins")]
use std::sync::{Arc, Mutex};

/// Action types for annotations created by plugins
#[cfg(feature = "plugins")]
#[derive(Debug, Clone)]
pub enum PendingAnnotationAction {
    AddHighlight { start: usize, end: usize, color: String },
    AddNote { start: usize, end: usize, text: String },
}

/// Snapshot of config values accessible to plugins
#[cfg(feature = "plugins")]
#[derive(Default, Clone)]
pub struct ConfigSnapshot {
    pub theme: String,
    pub hot_reload: bool,
    pub show_toc: bool,
    pub syntax_highlighting: bool,
}

/// Shared state for plugin API functions
#[cfg(feature = "plugins")]
#[derive(Default)]
pub struct PluginState {
    /// Current markdown content (shared with plugins)
    pub content: String,
    /// Pending notifications from plugins
    pub notifications: Vec<(String, String)>, // (message, level)
    /// Pending annotation actions from plugins
    pub pending_annotations: Vec<PendingAnnotationAction>,
    /// Pending config changes from plugins (key, value as string)
    pub pending_config_changes: Vec<(String, String)>,
    /// Current config snapshot (read-only for plugins, updated by app)
    pub config_snapshot: ConfigSnapshot,
}

#[cfg(feature = "plugins")]
impl PluginState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Register the mdview API in Lua
#[cfg(feature = "plugins")]
pub fn register_api(lua: &Lua, plugin_state: Arc<Mutex<PluginState>>) -> LuaResult<()> {
    let globals = lua.globals();

    // Create mdview namespace
    let mdview = lua.create_table()?;

    // Version info
    mdview.set("version", env!("CARGO_PKG_VERSION"))?;

    // Logging functions
    let log_info = lua.create_function(|_, msg: String| {
        log::info!("[plugin] {}", msg);
        Ok(())
    })?;
    mdview.set("log_info", log_info)?;

    let log_warn = lua.create_function(|_, msg: String| {
        log::warn!("[plugin] {}", msg);
        Ok(())
    })?;
    mdview.set("log_warn", log_warn)?;

    let log_error = lua.create_function(|_, msg: String| {
        log::error!("[plugin] {}", msg);
        Ok(())
    })?;
    mdview.set("log_error", log_error)?;

    // get_content() - returns the current markdown content
    let state_clone = Arc::clone(&plugin_state);
    let get_content = lua.create_function(move |_, ()| {
        match state_clone.lock() {
            Ok(state) => Ok(state.content.clone()),
            Err(_) => Err(mlua::Error::RuntimeError("Failed to acquire state lock".into())),
        }
    })?;
    mdview.set("get_content", get_content)?;

    // notify(msg, level) - show a notification in the status bar
    // level can be "info", "warn", or "error"
    let state_clone = Arc::clone(&plugin_state);
    let notify = lua.create_function(move |_, (msg, level): (String, Option<String>)| {
        let level = level.unwrap_or_else(|| "info".to_string());
        match state_clone.lock() {
            Ok(mut state) => {
                state.notifications.push((msg, level));
                Ok(())
            }
            Err(_) => Err(mlua::Error::RuntimeError("Failed to acquire state lock".into())),
        }
    })?;
    mdview.set("notify", notify)?;

    // add_highlight(start, end, color) - create a highlight annotation
    let state_clone = Arc::clone(&plugin_state);
    let add_highlight = lua.create_function(move |_, (start, end, color): (usize, usize, String)| {
        match state_clone.lock() {
            Ok(mut state) => {
                state.pending_annotations.push(PendingAnnotationAction::AddHighlight { start, end, color });
                Ok(())
            }
            Err(_) => Err(mlua::Error::RuntimeError("Failed to acquire state lock".into())),
        }
    })?;
    mdview.set("add_highlight", add_highlight)?;

    // add_note(start, end, text) - create a note annotation
    let state_clone = Arc::clone(&plugin_state);
    let add_note = lua.create_function(move |_, (start, end, text): (usize, usize, String)| {
        match state_clone.lock() {
            Ok(mut state) => {
                state.pending_annotations.push(PendingAnnotationAction::AddNote { start, end, text });
                Ok(())
            }
            Err(_) => Err(mlua::Error::RuntimeError("Failed to acquire state lock".into())),
        }
    })?;
    mdview.set("add_note", add_note)?;

    // get_setting(key) - read a config value
    let state_clone = Arc::clone(&plugin_state);
    let get_setting = lua.create_function(move |lua, key: String| {
        match state_clone.lock() {
            Ok(state) => {
                match key.as_str() {
                    "theme" => Ok(Value::String(lua.create_string(&state.config_snapshot.theme)?)),
                    "hot_reload" => Ok(Value::Boolean(state.config_snapshot.hot_reload)),
                    "show_toc" => Ok(Value::Boolean(state.config_snapshot.show_toc)),
                    "syntax_highlighting" => Ok(Value::Boolean(state.config_snapshot.syntax_highlighting)),
                    _ => Ok(Value::Nil),
                }
            }
            Err(_) => Err(mlua::Error::RuntimeError("Failed to acquire state lock".into())),
        }
    })?;
    mdview.set("get_setting", get_setting)?;

    // set_setting(key, value) - modify a config value
    let state_clone = Arc::clone(&plugin_state);
    let set_setting = lua.create_function(move |_, (key, value): (String, Value)| {
        let value_str = match value {
            Value::Boolean(b) => b.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.to_str()?.to_string(),
            _ => return Err(mlua::Error::RuntimeError("Unsupported value type".into())),
        };
        match state_clone.lock() {
            Ok(mut state) => {
                state.pending_config_changes.push((key, value_str));
                Ok(())
            }
            Err(_) => Err(mlua::Error::RuntimeError("Failed to acquire state lock".into())),
        }
    })?;
    mdview.set("set_setting", set_setting)?;

    // Register to globals
    globals.set("mdview", mdview)?;

    // Create hooks table
    let hooks = lua.create_table()?;
    globals.set("mdview_hooks", hooks)?;

    Ok(())
}

/// Available plugin hooks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginHook {
    /// Called when a file is opened
    OnFileOpen,
    /// Called when a file is closed
    OnFileClose,
    /// Called before rendering
    OnPreRender,
    /// Called after rendering
    OnPostRender,
    /// Called when theme changes
    OnThemeChange,
    /// Called when annotation is added
    OnAnnotationAdd,
    /// Called when annotation is removed
    OnAnnotationRemove,
}

impl PluginHook {
    /// Get the hook name as used in Lua
    pub fn lua_name(&self) -> &'static str {
        match self {
            PluginHook::OnFileOpen => "on_file_open",
            PluginHook::OnFileClose => "on_file_close",
            PluginHook::OnPreRender => "on_pre_render",
            PluginHook::OnPostRender => "on_post_render",
            PluginHook::OnThemeChange => "on_theme_change",
            PluginHook::OnAnnotationAdd => "on_annotation_add",
            PluginHook::OnAnnotationRemove => "on_annotation_remove",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_names() {
        assert_eq!(PluginHook::OnFileOpen.lua_name(), "on_file_open");
        assert_eq!(PluginHook::OnThemeChange.lua_name(), "on_theme_change");
    }
}
