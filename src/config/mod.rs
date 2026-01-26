//! Configuration module
//!
//! Handles loading, parsing, and managing application configuration.

pub mod defaults;
pub mod loader;
pub mod schema;

pub use schema::Config;
