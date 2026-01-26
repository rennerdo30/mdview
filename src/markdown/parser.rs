//! Markdown parser using pulldown-cmark

#![allow(dead_code)]

use pulldown_cmark::{Event, Options, Parser, Tag, HeadingLevel, CodeBlockKind};

use crate::config::Config;

/// Parse markdown content into events
pub fn parse(content: &str) -> Parser<'_> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;

    Parser::new_ext(content, options)
}

/// Parse markdown content into events with configuration
pub fn parse_with_config<'a>(content: &'a str, config: &Config) -> Parser<'a> {
    let mut options = Options::empty();

    if config.markdown.tables {
        options |= Options::ENABLE_TABLES;
    }
    if config.markdown.footnotes {
        options |= Options::ENABLE_FOOTNOTES;
    }
    if config.markdown.strikethrough {
        options |= Options::ENABLE_STRIKETHROUGH;
    }
    if config.markdown.task_lists {
        options |= Options::ENABLE_TASKLISTS;
    }
    if config.markdown.smart_punctuation {
        options |= Options::ENABLE_SMART_PUNCTUATION;
    }
    // Always enable heading attributes
    options |= Options::ENABLE_HEADING_ATTRIBUTES;

    Parser::new_ext(content, options)
}

/// Parse markdown with custom options
pub fn parse_with_options(content: &str, options: Options) -> Parser<'_> {
    Parser::new_ext(content, options)
}

/// Extract heading level as a number (1-6)
pub fn heading_level_to_usize(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Get the language from a code block kind
pub fn get_code_language<'a>(kind: &'a CodeBlockKind<'a>) -> Option<&'a str> {
    match kind {
        CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.as_ref()),
        _ => None,
    }
}

/// Check if an event is a block-level element
pub fn is_block_element(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::Start(Tag::Paragraph)
            | Event::Start(Tag::Heading { .. })
            | Event::Start(Tag::BlockQuote(_))
            | Event::Start(Tag::CodeBlock(_))
            | Event::Start(Tag::List(_))
            | Event::Start(Tag::Item)
            | Event::Start(Tag::Table(_))
            | Event::Start(Tag::TableHead)
            | Event::Start(Tag::TableRow)
            | Event::Rule
    )
}

/// Check if an event starts a new section (heading)
pub fn is_section_start(event: &Event<'_>) -> bool {
    matches!(event, Event::Start(Tag::Heading { .. }))
}

/// Collect text content from a range of events
pub fn collect_text<'a>(events: impl Iterator<Item = &'a Event<'a>>) -> String {
    let mut text = String::new();
    for event in events {
        if let Event::Text(t) | Event::Code(t) = event {
            text.push_str(t);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let content = "# Hello\n\nWorld";
        let events: Vec<_> = parse(content).collect();
        assert!(!events.is_empty());
    }

    #[test]
    fn test_heading_level() {
        assert_eq!(heading_level_to_usize(HeadingLevel::H1), 1);
        assert_eq!(heading_level_to_usize(HeadingLevel::H6), 6);
    }
}
