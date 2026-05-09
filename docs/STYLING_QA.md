# Styling and Visual QA

Use the fixtures in `fixtures/markdown` for Markdown styling checks:

- `dense.md`: every supported Markdown family in one document.
- `typography.md`: heading hierarchy, paragraph rhythm, and long-form reading.
- `edge-cases.md`: long words, long URLs, wide tables, missing images, nested quotes, and deeply nested lists.

## Core UI States

Capture or inspect these states after significant renderer or theme changes:

- Welcome screen.
- Dense document with table of contents open.
- Dense document with file browser open.
- Annotation selection and annotation popup.
- Code blocks with line numbers on and off.
- Light and dark themes.
- Settings dialog.
- Find-in-document bar with a highlighted match.

## Styling Matrix

For each fixture, check:

- Themes: light, dark, sepia or custom if configured, and high contrast if configured.
- Reading widths: narrow, comfortable, and full width.
- Font sizes: minimum, default, and maximum.
- Image width: limited and unlimited.

## Markdown Elements

Look for overlap, clipping, unstable layout shifts, unreadable contrast, and broken wrapping in:

- Heading hierarchy and spacing.
- Paragraph line height and rhythm.
- Ordered, unordered, nested, and task lists.
- Blockquotes and nested blockquotes.
- Tables with wide cells.
- Fenced code blocks, inline code, and code in list items.
- Links, long URLs, and links with inline code.
- Images, missing images, and large images.
- Footnotes and horizontal rules.
- Mixed inline styles such as bold plus italic, strikethrough, annotations, and inline code.

## Known Fix Policy

Treat these as regressions:

- Text overlaps another element.
- Text clips inside a button, table cell, code block, or list item.
- Long words or URLs force controls off-screen.
- Code block line numbers or text have insufficient contrast.
- Images overflow the content column or cause repeated layout jumps.
- Search highlight, annotation highlight, or link styling makes text unreadable.

When a regression is found, add or update a fixture first, then patch the renderer or style rule.
