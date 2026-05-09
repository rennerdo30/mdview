//! Markdown parser using pulldown-cmark

#![allow(dead_code)]

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};

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
    if config.markdown.math {
        options |= Options::ENABLE_MATH;
    }
    if config.markdown.metadata_blocks {
        options |= Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
        options |= Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    }
    if config.markdown.definition_lists {
        options |= Options::ENABLE_DEFINITION_LIST;
    }
    if config.markdown.old_footnotes {
        options |= Options::ENABLE_OLD_FOOTNOTES;
    }
    if config.markdown.gfm {
        options |= Options::ENABLE_GFM;
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
    use pulldown_cmark::{BlockQuoteKind, MetadataBlockKind, TagEnd};

    fn parse_default_events(content: &str) -> Vec<Event<'_>> {
        let config = Config::default();
        parse_with_config(content, &config).collect()
    }

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

    #[test]
    fn test_parse_extended_markdown_events() {
        let content = r#"---
title: Demo
---

Term
: Definition

Inline math $a + b$.

$$
x = y
$$

<section>Raw HTML</section>
"#;
        let events = parse_default_events(content);

        assert!(events.iter().any(|event| matches!(
            event,
            Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle))
        )));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::DefinitionList))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::InlineMath(_))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::DisplayMath(_))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Html(_) | Event::InlineHtml(_))));
    }

    #[test]
    fn test_parse_core_commonmark_fixture() {
        let content = r#"# Heading

Paragraph with **strong**, *emphasis*, `code`, and [a link](https://example.com).

> Quote

- first
- second

1. ordered
2. list

---
"#;
        let events = parse_default_events(content);

        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Heading { .. }))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Strong))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Emphasis))));
        assert!(events.iter().any(|event| matches!(event, Event::Code(_))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Link { .. }))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::BlockQuote(_)))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::List(None)))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::List(Some(1))))));
        assert!(events.iter().any(|event| matches!(event, Event::Rule)));
    }

    #[test]
    fn test_parse_gfm_like_fixture() {
        let content = r#"| Name | Done |
| ---- | ---- |
| Task | yes |

- [x] complete
- [ ] pending

~~deleted~~

[^note]

[^note]: footnote text
"#;
        let events = parse_default_events(content);

        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Table(_)))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::TableHead))));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TaskListMarker(_)))
                .count(),
            2
        );
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Strikethrough))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::FootnoteReference(_))));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::FootnoteDefinition(_)))));
    }

    #[test]
    fn test_parse_links_images_and_code_fences_fixture() {
        let content = r#"![Alt text](images/example.png)

```rust
fn main() {}
```

<https://example.com>
"#;
        let events = parse_default_events(content);

        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Image { .. }))));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) if language.as_ref() == "rust"
            )
        }));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Link { .. }))));
    }

    #[test]
    fn test_parse_metadata_variants_fixture() {
        let yaml_events = parse_default_events(
            r#"---
title: YAML
---
"#,
        );
        let plus_events = parse_default_events(
            r#"+++
title = "Plus"
+++
"#,
        );

        assert!(yaml_events.iter().any(|event| {
            matches!(
                event,
                Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle))
            )
        }));
        assert!(plus_events.iter().any(|event| {
            matches!(
                event,
                Event::Start(Tag::MetadataBlock(MetadataBlockKind::PlusesStyle))
            )
        }));
    }

    #[test]
    fn test_parse_config_flags_can_disable_extensions() {
        let mut config = Config::default();
        config.markdown.tables = false;
        config.markdown.task_lists = false;
        config.markdown.strikethrough = false;
        config.markdown.math = false;
        config.markdown.metadata_blocks = false;
        config.markdown.definition_lists = false;

        let content = r#"---
title: Demo
---

| A |
| - |
| B |

- [x] task

~~deleted~~

$x$

Term
: Definition
"#;
        let events: Vec<_> = parse_with_config(content, &config).collect();

        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Table(_)))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::TaskListMarker(_))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::Strikethrough))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::InlineMath(_))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::MetadataBlock(_)))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::DefinitionList))));
    }

    #[test]
    fn test_gfm_flag_only_adds_bundled_gfm_extensions() {
        let content = "> [!NOTE]\n> Pay attention.\n";
        let mut config = Config::default();

        let default_events: Vec<_> = parse_with_config(content, &config).collect();
        config.markdown.gfm = true;
        let gfm_events: Vec<_> = parse_with_config(content, &config).collect();

        assert!(default_events
            .iter()
            .any(|event| matches!(event, Event::Start(Tag::BlockQuote(None)))));
        assert!(gfm_events.iter().any(|event| matches!(
            event,
            Event::Start(Tag::BlockQuote(Some(BlockQuoteKind::Note)))
        )));
    }

    #[test]
    fn test_old_footnotes_are_opt_in() {
        let content = "Known[^known] missing[^missing]\n\n[^known]: footnote\n";
        let mut config = Config::default();

        let default_refs = parse_with_config(content, &config)
            .filter(|event| matches!(event, Event::FootnoteReference(_)))
            .count();
        config.markdown.old_footnotes = true;
        let old_refs = parse_with_config(content, &config)
            .filter(|event| matches!(event, Event::FootnoteReference(_)))
            .count();
        let old_definitions = parse_with_config(content, &config)
            .filter(|event| matches!(event, Event::Start(Tag::FootnoteDefinition(_))))
            .count();

        assert_eq!(default_refs, 1);
        assert_eq!(old_refs, 2);
        assert_eq!(old_definitions, 1);
    }

    #[test]
    fn test_collect_text_covers_text_and_inline_code() {
        let events = parse_default_events("Text with `code`.");

        assert_eq!(collect_text(events.iter()), "Text with code.");
    }

    #[test]
    fn test_is_block_element_tracks_supported_blocks() {
        let events = parse_default_events(
            r#"# Heading

Paragraph

| A |
| - |
| B |
"#,
        );

        assert!(events.iter().any(is_block_element));
        assert!(events.iter().any(is_section_start));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::End(TagEnd::Table))));
    }
}
