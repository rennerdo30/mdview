//! Plugin module (feature-gated)
//!
//! Provides Lua plugin support for extending mdview functionality.

#[cfg(feature = "plugins")]
pub mod api;
#[cfg(feature = "plugins")]
pub mod lua_runtime;

#[cfg(feature = "plugins")]
pub use lua_runtime::LuaRuntime;
