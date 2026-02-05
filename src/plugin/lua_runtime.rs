//! Lua plugin runtime

#[cfg(feature = "plugins")]
use mlua::{Lua, Result as LuaResult, Function, Table, Value};

#[cfg(feature = "plugins")]
use std::sync::{Arc, Mutex};

use std::path::Path;

#[cfg(feature = "plugins")]
pub use super::api::PluginState;

/// Lua plugin runtime
#[cfg(feature = "plugins")]
pub struct LuaRuntime {
    lua: Lua,
    /// Shared state for plugin API
    pub state: Arc<Mutex<PluginState>>,
}

#[cfg(feature = "plugins")]
impl LuaRuntime {
    /// Create a new Lua runtime with sandboxed environment
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let state = Arc::new(Mutex::new(PluginState::new()));

        // Set up sandboxed environment — remove dangerous globals that could
        // escape the sandbox via filesystem, code loading, or introspection.
        lua.scope(|_scope| {
            let globals = lua.globals();

            // Remove file I/O
            globals.set("io", Value::Nil)?;
            globals.set("dofile", Value::Nil)?;
            globals.set("loadfile", Value::Nil)?;

            // Remove OS functions
            globals.set("os", Value::Nil)?;

            // Remove debug library (allows metatable/upvalue introspection)
            globals.set("debug", Value::Nil)?;

            // Remove code loading functions that could bypass sandbox
            globals.set("require", Value::Nil)?;
            globals.set("loadstring", Value::Nil)?;
            globals.set("load", Value::Nil)?;
            globals.set("rawget", Value::Nil)?;
            globals.set("rawset", Value::Nil)?;

            // Remove package module (used by require)
            globals.set("package", Value::Nil)?;

            // Keep safe functions: string, table, math, print, pairs, ipairs, etc.

            Ok(())
        })?;

        // Register mdview API with shared state
        super::api::register_api(&lua, Arc::clone(&state))?;

        Ok(Self { lua, state })
    }

    /// Load and execute a plugin file
    pub fn load_plugin(&self, path: &Path) -> LuaResult<()> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| mlua::Error::ExternalError(std::sync::Arc::new(e)))?;

        self.lua.load(&code).exec()?;

        Ok(())
    }

    /// Call a plugin hook function
    pub fn call_hook(&self, hook_name: &str, args: impl mlua::IntoLuaMulti) -> LuaResult<()> {
        let globals = self.lua.globals();

        if let Ok(hooks) = globals.get::<Table>("mdview_hooks") {
            if let Ok(hook) = hooks.get::<Function>(hook_name) {
                hook.call::<()>(args)?;
            }
        }

        Ok(())
    }

    /// Get the Lua instance
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Update the content available to plugins
    pub fn set_content(&self, content: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.content = content.to_string();
        }
    }

    /// Check if there are pending notifications (quick check without taking lock for long)
    pub fn has_pending_notifications(&self) -> bool {
        self.state.lock().map(|s| !s.notifications.is_empty()).unwrap_or(false)
    }

    /// Get and clear pending notifications from plugins
    pub fn take_notifications(&self) -> Vec<(String, String)> {
        if let Ok(mut state) = self.state.lock() {
            std::mem::take(&mut state.notifications)
        } else {
            Vec::new()
        }
    }

    /// Update the config snapshot available to plugins
    pub fn update_config_snapshot(&self, config: &crate::config::Config) {
        if let Ok(mut state) = self.state.lock() {
            state.config_snapshot = super::api::ConfigSnapshot {
                theme: config.general.theme.clone(),
                hot_reload: config.general.hot_reload,
                show_toc: config.general.show_toc,
                syntax_highlighting: config.markdown.syntax_highlighting,
            };
        }
    }

    /// Check if there are pending annotation actions
    pub fn has_pending_annotations(&self) -> bool {
        self.state.lock().map(|s| !s.pending_annotations.is_empty()).unwrap_or(false)
    }

    /// Get and clear pending annotation actions from plugins
    pub fn take_pending_annotations(&self) -> Vec<super::api::PendingAnnotationAction> {
        if let Ok(mut state) = self.state.lock() {
            std::mem::take(&mut state.pending_annotations)
        } else {
            Vec::new()
        }
    }

    /// Check if there are pending config changes
    pub fn has_pending_config_changes(&self) -> bool {
        self.state.lock().map(|s| !s.pending_config_changes.is_empty()).unwrap_or(false)
    }

    /// Get and clear pending config changes from plugins
    pub fn take_pending_config_changes(&self) -> Vec<(String, String)> {
        if let Ok(mut state) = self.state.lock() {
            std::mem::take(&mut state.pending_config_changes)
        } else {
            Vec::new()
        }
    }
}

// Note: Default impl removed - use new() directly which returns Result
// This avoids the panic from expect() and allows proper error handling

#[cfg(not(feature = "plugins"))]
pub struct LuaRuntime;

#[cfg(not(feature = "plugins"))]
impl LuaRuntime {
    pub fn new() -> Result<Self, &'static str> {
        Err("Plugins feature not enabled")
    }
}
