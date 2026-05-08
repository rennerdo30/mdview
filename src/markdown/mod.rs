//! Markdown parsing and rendering module

use std::path::Path;

pub mod mermaid;
pub mod parser;
pub mod renderer;

/// File extensions that mdview treats as Markdown-like documents.
///
/// `.mdx` is included for pragmatic viewing of Markdown portions only; mdview does
/// not execute or render MDX JSX/ESM as components.
pub const MARKDOWN_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdown", "mdtxt", "mdwn", "mkd", "mkdn", "mdx", "qmd",
];

/// Return true when a path has a known Markdown-like extension.
pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            MARKDOWN_EXTENSIONS
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

// Public API re-exports
#[allow(unused_imports)]
pub use parser::parse;
#[allow(unused_imports)]
pub use renderer::MarkdownRenderer;
