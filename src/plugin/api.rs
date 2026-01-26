//! Plugin API for Lua scripts

#[cfg(feature = "plugins")]
use mlua::{Lua, Result as LuaResult, Function, Table, Value};

/// Register the mdview API in Lua
#[cfg(feature = "plugins")]
pub fn register_api(lua: &Lua) -> LuaResult<()> {
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
