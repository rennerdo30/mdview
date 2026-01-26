//! Lua plugin runtime

#[cfg(feature = "plugins")]
use mlua::{Lua, Result as LuaResult, Function, Table, Value};

use std::path::Path;

/// Lua plugin runtime
#[cfg(feature = "plugins")]
pub struct LuaRuntime {
    lua: Lua,
}

#[cfg(feature = "plugins")]
impl LuaRuntime {
    /// Create a new Lua runtime with sandboxed environment
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();

        // Set up sandboxed environment
        lua.scope(|scope| {
            // Remove potentially dangerous functions
            let globals = lua.globals();

            // Remove file I/O
            globals.set("io", Value::Nil)?;
            globals.set("dofile", Value::Nil)?;
            globals.set("loadfile", Value::Nil)?;

            // Remove OS functions
            globals.set("os", Value::Nil)?;

            // Remove debug library
            globals.set("debug", Value::Nil)?;

            // Keep safe functions: string, table, math, etc.

            Ok(())
        })?;

        // Register mdview API
        super::api::register_api(&lua)?;

        Ok(Self { lua })
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
                hook.call(args)?;
            }
        }

        Ok(())
    }

    /// Get the Lua instance
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

#[cfg(feature = "plugins")]
impl Default for LuaRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create Lua runtime")
    }
}

#[cfg(not(feature = "plugins"))]
pub struct LuaRuntime;

#[cfg(not(feature = "plugins"))]
impl LuaRuntime {
    pub fn new() -> Result<Self, &'static str> {
        Err("Plugins feature not enabled")
    }
}
