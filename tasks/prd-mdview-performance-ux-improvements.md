# PRD: mdview Performance & UX Improvements

## Overview
mdview feels sluggish when rendering large markdown files (2000-10000+ lines), and the navigation/visual experience needs polish. This PRD covers performance optimizations for initial load and scrolling, plus UX improvements for TOC navigation, typography, transitions, and syntax highlighting.

## Goals
- Reduce perceived rendering time for large markdown files (2000-10000+ lines)
- Eliminate scroll stutter/lag on large documents
- Add search/filter to the TOC panel for quick navigation
- Improve typography hierarchy and readability
- Add smooth transitions throughout the UI
- Improve syntax highlighting quality and coverage

## Quality Gates

These commands must pass for every user story:
- `cargo test` - All tests pass
- `cargo clippy` - No lint warnings
- `cargo check` - No compiler warnings

## User Stories

### US-001: Profile and optimize initial markdown rendering
As a user, I want large markdown files (5000+ lines) to render quickly so that I don't wait when opening documents.

**Acceptance Criteria:**
- [x] Profile the current rendering pipeline to identify bottlenecks
- [x] Optimize the pulldown-cmark event processing in `markdown/renderer.rs`
- [x] Reduce unnecessary widget allocations during initial render
- [x] Large files (5000+ lines) should feel noticeably faster to open

### US-002: Optimize scrolling performance for large documents
As a user, I want smooth scrolling through large rendered documents without stutter or frame drops.

**Acceptance Criteria:**
- [x] Profile scroll performance with a 5000+ line rendered document
- [x] Reduce per-frame work during scroll (avoid re-layout of off-screen content where possible)
- [x] Ensure egui repaints are efficient (minimize unnecessary repaints)
- [x] Scrolling feels smooth on documents with 10000+ lines

### US-003: Implement viewport-aware rendering
As a user, I want the app to prioritize rendering what's visible on screen so that perceived performance is optimal regardless of file size.

**Acceptance Criteria:**
- [x] Only render markdown elements that are in or near the current viewport
- [x] Off-screen content is rendered lazily as the user scrolls
- [x] Scroll position and document height remain accurate
- [x] Jumping to a TOC heading still works correctly with lazy rendering

### US-004: Add search/filter to the TOC panel
As a user, I want to search within the table of contents so that I can quickly find sections in large documents.

**Acceptance Criteria:**
- [x] Add a text input field at the top of the TOC panel
- [x] Typing filters headings in real-time (whatever feels most natural — fuzzy or substring)
- [x] Non-matching headings are hidden or dimmed
- [x] Clearing the search restores the full TOC
- [x] Clicking a filtered result navigates to that section
- [x] Filter field is accessible via keyboard shortcut

### US-005: Improve typography hierarchy and readability
As a user, I want clear visual distinction between heading levels and comfortable reading spacing so that documents are easy to scan and read.

**Acceptance Criteria:**
- [x] Increase size/weight contrast between H1-H6 headings
- [x] Improve line height and paragraph spacing for body text
- [x] Add configurable font size (zoom in/out via keyboard shortcut or menu)
- [x] Typography changes respect the current theme (light/dark)
- [x] Zoom level persists in config

### US-006: Add smooth scrolling for TOC navigation
As a user, I want smooth animated scrolling when clicking a TOC entry so that I maintain spatial context.

**Acceptance Criteria:**
- [x] Clicking a TOC entry smoothly scrolls to the target heading (not an instant jump)
- [x] Animation duration is reasonable (~200-400ms)
- [x] Smooth scroll can be interrupted by user input (manual scroll, another click)
- [x] Works correctly with viewport-aware rendering (US-003)

### US-007: Add animated TOC panel open/close
As a user, I want the TOC panel to open and close with a smooth animation so the UI feels polished.

**Acceptance Criteria:**
- [x] TOC panel slides in/out when toggled
- [x] Animation is smooth and doesn't cause layout jank
- [x] Content area resizes smoothly alongside the panel
- [x] Animation duration is short (~150-250ms)

### US-008: Add fade transitions when switching files
As a user, I want a subtle fade transition when switching between files so the experience feels smooth rather than jarring.

**Acceptance Criteria:**
- [x] Switching files shows a brief fade-out/fade-in transition
- [x] Transition is fast enough to not feel slow (~150-200ms)
- [x] Transition does not block user input
- [x] Works correctly with file browser and recent files

### US-009: Improve syntax highlighting in code blocks
As a user, I want better syntax highlighting with more language support, theme-aware colors, and line numbers so that code blocks are easy to read.

**Acceptance Criteria:**
- [x] Add support for more languages (at minimum: Rust, Python, JavaScript, TypeScript, Go, C/C++, Java, TOML, YAML, JSON, Bash, SQL)
- [x] Syntax highlighting colors adapt to the current theme (light/dark)
- [x] Add optional line numbers in code blocks
- [x] Line numbers can be toggled via config setting
- [x] Code block rendering performance does not regress

## Functional Requirements
- FR-1: The rendering pipeline must handle files of 10000+ lines without noticeable lag on initial open
- FR-2: Scrolling must maintain 60fps on documents of 5000+ lines
- FR-3: Viewport-aware rendering must not break scroll position accuracy or TOC navigation
- FR-4: TOC search must filter results in real-time as the user types
- FR-5: Font size must be adjustable via Ctrl+/Ctrl- (or Cmd+/Cmd- on macOS) keyboard shortcuts
- FR-6: Zoom level must persist in the TOML config file
- FR-7: Smooth scroll animations must not block user input
- FR-8: Syntax highlighting must support at least 12 common languages
- FR-9: Line numbers in code blocks must be toggleable via config
- FR-10: All visual changes must work correctly in both light and dark themes

## Non-Goals
- No changes to the plugin system
- No changes to the config file format (only additive fields)
- No changes to PDF export functionality
- No custom theme editor or theme creation tools
- No AST-based rendering rewrite (optimize within current streaming approach)

## Technical Considerations
- **Viewport-aware rendering:** egui doesn't have native virtualization — may need to calculate element heights and skip rendering off-screen items in the scroll area
- **Smooth scrolling:** egui's `ScrollArea` supports programmatic scroll offset — animate by interpolating offset over frames
- **Syntax highlighting:** Consider `syntect` crate if not already used; ensure it integrates with theme colors
- **Font size zoom:** Store in `AppState`, apply as a multiplier to base font sizes in the theme
- **Performance profiling:** Use `tracing` or `puffin` for flame graphs to identify actual bottlenecks before optimizing

## Success Metrics
- Large file (5000+ lines) opens without perceptible delay
- Scrolling is smooth (no visible stutter) on 10000+ line documents
- TOC search finds target headings within 1-2 keystrokes
- Typography is visually distinct across all heading levels
- All transitions feel smooth and polished
- Code blocks are readable with proper highlighting in both themes

## Open Questions
- Should viewport-aware rendering use estimated heights or pre-calculated heights for off-screen elements?
- What is the current syntax highlighting implementation — is `syntect` already in use or would it need to be added?
- Should zoom level affect only body text or also headings and code blocks proportionally?