//! TOC builder - extracts headings from markdown

#![allow(dead_code)]

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::markdown::parser;

/// A single entry in the table of contents
#[derive(Debug, Clone)]
pub struct TocEntry {
    /// Heading text
    pub text: String,

    /// Pre-lowercased text for efficient case-insensitive search (computed once)
    pub text_lower: String,

    /// Heading level (1-6)
    pub level: usize,

    /// Index in the flat list of headings
    pub index: usize,

    /// Nested children (for tree view)
    pub children: Vec<TocEntry>,
}

impl TocEntry {
    pub fn new(text: String, level: usize, index: usize) -> Self {
        let text_lower = text.to_lowercase();
        Self {
            text,
            text_lower,
            level,
            index,
            children: Vec::new(),
        }
    }
}

/// Table of contents tree structure
#[derive(Debug, Clone, Default)]
pub struct TocTree {
    /// Top-level entries
    pub entries: Vec<TocEntry>,

    /// Flat list of all entries for quick lookup
    pub flat: Vec<TocEntry>,
}

impl TocTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if TOC is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get total count of headings
    pub fn len(&self) -> usize {
        self.flat.len()
    }

    /// Get entry by index
    pub fn get(&self, index: usize) -> Option<&TocEntry> {
        self.flat.get(index)
    }
}

/// Build a TOC tree from pre-parsed markdown events (more efficient - avoids re-parsing)
pub fn build_toc_from_events<'a>(events: impl IntoIterator<Item = &'a Event<'a>>) -> TocTree {
    let mut flat_entries = Vec::new();
    let mut current_text = String::new();
    let mut current_level = 0;
    let mut in_heading = false;

    for event in events {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_level = parser::heading_level_to_usize(*level);
                current_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if in_heading && !current_text.is_empty() {
                    let index = flat_entries.len();
                    flat_entries.push(TocEntry::new(
                        current_text.trim().to_string(),
                        current_level,
                        index,
                    ));
                }
                in_heading = false;
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                current_text.push_str(text);
            }
            _ => {}
        }
    }

    // Build tree structure
    let tree_entries = build_tree(&flat_entries);

    TocTree {
        entries: tree_entries,
        flat: flat_entries,
    }
}

/// Build a TOC tree from markdown content (convenience function, less efficient)
pub fn build_toc(content: &str) -> TocTree {
    let parser_events: Vec<_> = parser::parse(content).collect();
    build_toc_from_events(&parser_events)
}

/// Build a nested tree from flat entries
fn build_tree(flat: &[TocEntry]) -> Vec<TocEntry> {
    if flat.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<TocEntry> = Vec::new();
    // Stack stores (entry, level) pairs where entry is being built
    // We use indices to track where to insert children
    let mut stack: Vec<(TocEntry, usize)> = Vec::new();

    for entry in flat {
        let mut new_entry = entry.clone();
        new_entry.children = Vec::new();
        let level = entry.level;

        // Pop entries with level >= current level and attach them to their parents
        while let Some((_, top_level)) = stack.last() {
            if *top_level < level {
                // Found a parent, stop popping
                break;
            }
            // Pop this entry and add to parent or result
            if let Some((popped, _)) = stack.pop() {
                if let Some((parent, _)) = stack.last_mut() {
                    parent.children.push(popped);
                } else {
                    result.push(popped);
                }
            }
        }

        stack.push((new_entry, level));
    }

    // Empty remaining stack - process from top to bottom (reverse order)
    while let Some((entry, _)) = stack.pop() {
        if let Some((parent, _)) = stack.last_mut() {
            parent.children.push(entry);
        } else {
            result.push(entry);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_toc_basic() {
        let content = "# Heading 1\n\n## Heading 2\n\n### Heading 3\n\n## Another H2";
        let toc = build_toc(content);

        assert_eq!(toc.len(), 4);
        assert_eq!(toc.flat[0].text, "Heading 1");
        assert_eq!(toc.flat[0].level, 1);
        assert_eq!(toc.flat[1].text, "Heading 2");
        assert_eq!(toc.flat[1].level, 2);
    }

    #[test]
    fn test_build_toc_empty() {
        let content = "No headings here, just text.";
        let toc = build_toc(content);

        assert!(toc.is_empty());
    }

    #[test]
    fn test_build_toc_tree_structure() {
        let content = "# H1\n\n## H2a\n\n### H3\n\n## H2b";
        let toc = build_toc(content);

        // Test flat structure is correct
        assert_eq!(toc.len(), 4);
        assert_eq!(toc.flat[0].text, "H1");
        assert_eq!(toc.flat[0].level, 1);
        assert_eq!(toc.flat[1].text, "H2a");
        assert_eq!(toc.flat[1].level, 2);
        assert_eq!(toc.flat[2].text, "H3");
        assert_eq!(toc.flat[2].level, 3);
        assert_eq!(toc.flat[3].text, "H2b");
        assert_eq!(toc.flat[3].level, 2);

        // Test tree structure - H1 should be the only root
        assert_eq!(toc.entries.len(), 1);
        assert_eq!(toc.entries[0].text, "H1");

        // H1 should have two children: H2a and H2b in that order
        assert_eq!(toc.entries[0].children.len(), 2);
        assert_eq!(toc.entries[0].children[0].text, "H2a");
        assert_eq!(toc.entries[0].children[1].text, "H2b");

        // H2a should have one child: H3
        assert_eq!(toc.entries[0].children[0].children.len(), 1);
        assert_eq!(toc.entries[0].children[0].children[0].text, "H3");

        // H2b should have no children
        assert_eq!(toc.entries[0].children[1].children.len(), 0);
    }

    #[test]
    fn test_build_toc_preserves_document_order() {
        // Test with multiple siblings at the same level
        let content = "# Doc\n\n## Section A\n\n## Section B\n\n## Section C";
        let toc = build_toc(content);

        assert_eq!(toc.entries.len(), 1);
        assert_eq!(toc.entries[0].children.len(), 3);

        // Children should be in document order: A, B, C
        assert_eq!(toc.entries[0].children[0].text, "Section A");
        assert_eq!(toc.entries[0].children[1].text, "Section B");
        assert_eq!(toc.entries[0].children[2].text, "Section C");
    }
}
