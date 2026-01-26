//! Markdown parsing and rendering module

pub mod mermaid;
pub mod parser;
pub mod renderer;

// Public API re-exports
#[allow(unused_imports)]
pub use parser::parse;
#[allow(unused_imports)]
pub use renderer::MarkdownRenderer;
