# mdview TODO

## Completed

### Performance Optimizations (Scroll Lag Fix)
- [x] AnnotationIndex activation: O(log n) binary search for annotation lookups
- [x] Syntax highlighting cache: LRU cache (100 entries) for code block LayoutJobs
- [x] String allocation reduction: removed `.collect()` in render hot paths
- [x] TOC event reuse: `build_toc_from_events()` eliminates duplicate parsing
- [x] Mermaid preview metadata cache: avoid re-parsing diagram source each frame

### Code Quality & Bug Fixes
- [x] Fix `current_theme()` method call in viewer.rs (was missing parentheses)
- [x] LRU eviction for image cache (max 50 entries)
- [x] Safe unwrap in theme system (returns None instead of panicking)
- [x] Dead code removal: 10 unused theme color functions

### Previous
- [x] Async mermaid rendering (background threads, loading spinner)
- [x] Native macOS/Windows/Linux menu bar (muda crate)
- [x] Windows font fallback chain expansion
- [x] File association error handling improvements
- [x] PDF blockquote rendering with background
- [x] Annotation position estimation improvements
- [x] File watcher event differentiation (modified vs removed)
- [x] Recent files path canonicalization logging
- [x] Dead code cleanup (mermaid helpers)
- [x] Mermaid CLI fallback when native fails
- [x] PDF export: true dark theme with page background color
- [x] Annotation: visible character range tracking during render

## In Progress

None

## Planned

### P0: Markdown Detection & Dialect Correctness
- [ ] Expand markdown file detection beyond `.md`, `.markdown`, and `.mdx`.
  - Include common variants: `.mkd`, `.mkdn`, `.mdown`, `.mdwn`, `.mdtxt`, and optionally `.qmd`.
  - Keep detection case-insensitive.
  - Update file browser tests and open-dialog filters.
- [ ] Decide and document MDX support policy.
  - Either remove `.mdx` from the markdown filter, or display MDX as Markdown-with-unsupported-JSX clearly.
  - Do not imply full MDX rendering unless JSX/ESM parsing is implemented.
- [ ] Add a parser/rendering coverage test suite for core Markdown fixtures.
  - Cover CommonMark basics, GFM-like features, front matter, raw HTML, math, definition lists, links, images, tables, footnotes, and task lists.
  - Include renderer smoke tests where practical so parsed events are not silently ignored.

### P0: Markdown Rendering Gaps
- [ ] Render raw HTML and inline HTML events safely.
  - Decide whether to show sanitized text, a styled fallback block, or a safe HTML subset.
  - Ensure HTML does not execute scripts or load unsafe resources.
- [ ] Add math support.
  - Enable `pulldown-cmark` math parsing.
  - Render inline and block math with a readable fallback first; consider a real math renderer later.
  - Add PDF export fallback for math.
- [ ] Add front matter / metadata block support.
  - Enable YAML-style metadata blocks.
  - Enable plus-delimited metadata blocks if useful.
  - Render metadata as a collapsible/info block or hide it behind a config option.
- [ ] Add definition list support.
  - Enable `ENABLE_DEFINITION_LIST`.
  - Render terms and definitions with clear spacing and indentation.
  - Add PDF export support.
- [ ] Evaluate `ENABLE_GFM` versus individually enabled GFM features.
  - Confirm whether enabling full GFM changes behavior unexpectedly.
  - Add regression fixtures for tables, task lists, strikethrough, and autolinks.
- [ ] Decide whether to support old footnote syntax.
  - If enabled, add compatibility tests and document the behavior.

### P1: PDF Export Coverage
- [ ] Align PDF export with all supported Markdown renderer features.
  - Raw HTML fallback.
  - Math fallback.
  - Metadata/front matter handling.
  - Definition lists.
  - Any future GFM-specific behavior.
- [ ] Expand PDF image support.
  - Add SVG support through rasterization or documented fallback.
  - Add WebP support if the image stack supports it reliably.
  - Keep local-path traversal protections intact.

### P1: Performance & Regression Gates
- [ ] Add repeatable performance benchmarks.
  - Benchmark initial load for 1k, 5k, and 10k line documents.
  - Benchmark scroll/render hot paths with large tables, code blocks, images, annotations, and TOC-heavy files.
  - Track parse time, render frame time, memory, and cache hit behavior.
- [ ] Add fixture-based performance regression thresholds.
  - Keep thresholds loose enough for CI variance but strict enough to catch obvious regressions.
  - Document how to run benchmarks locally.
- [ ] Add stress fixtures for huge Markdown documents.
  - Large plain document.
  - Large document with many headings.
  - Large document with many annotations.
  - Large document with many fenced code blocks and images.
- [ ] Audit remaining per-frame work.
  - Confirm heading positions, visible range calculation, plugin hooks, and annotation selection do not scale poorly on large files.
  - Profile before further optimization.

### P1: Search & Navigation
- [ ] Add full-document search.
  - Search body text, headings, code blocks, and table text.
  - Provide next/previous navigation.
  - Highlight matches without tanking scroll performance.
  - Integrate with TOC search where appropriate.
- [ ] Add search result indexing/cache invalidation.
  - Invalidate on content change and relevant config changes.
  - Avoid rebuilding indexes every frame.

### P1: Configuration & UX
- [ ] Add a settings/preferences UI.
  - General settings: theme, hot reload, TOC visibility, file browser behavior.
  - Markdown settings: extensions, syntax highlighting, line numbers, math, HTML, metadata visibility.
  - Layout settings: font size, reading width, image width.
  - Export settings: PDF page size, margins, TOC, theme.
- [ ] Make unsupported feature fallbacks visible but unobtrusive.
  - MDX JSX/ESM fallback.
  - Unsupported image types.
  - Mermaid CLI missing.
  - Math renderer unavailable.

### P2: Syntax Highlighting Coverage
- [ ] Add syntax language coverage tests.
  - Verify common aliases for Rust, Python, JavaScript, TypeScript, TSX, JSX, Go, C, C++, Java, TOML, YAML, JSON, Bash, SQL, HTML, CSS, Markdown, and diff.
  - Confirm unknown languages fall back to plain text.
- [ ] Normalize common language aliases before passing to syntect.
  - Examples: `sh`/`shell`/`bash`, `js`/`javascript`, `ts`/`typescript`, `yml`/`yaml`, `c++`/`cpp`.

### P2: Visual & GUI Regression Testing
- [ ] Add visual regression/screenshot tests for core UI states.
  - Welcome screen.
  - Large document with TOC.
  - File browser.
  - Annotations.
  - Code blocks with line numbers.
  - Light/dark themes.
- [ ] Add PDF visual smoke tests.
  - Generate small PDFs for fixture documents.
  - Validate that export succeeds and includes expected high-level content.

### P2: Markdown Styling & Layout QA
- [ ] Audit rendered Markdown styling against real documents.
  - Check heading hierarchy, spacing, line height, paragraph rhythm, and nested block spacing.
  - Check lists, nested lists, task lists, blockquotes, tables, code blocks, inline code, links, images, footnotes, and horizontal rules.
  - Check mixed inline styles such as bold plus italic, links with inline code, strikethrough with annotations, and code inside list items.
- [ ] Create styling regression fixtures.
  - One dense document with every supported Markdown element.
  - One typography-heavy document with long paragraphs and many heading levels.
  - One edge-case document with long words, long URLs, wide tables, large images, and deeply nested lists.
- [ ] Verify styling across themes and layout settings.
  - Light, dark, sepia, high-contrast, and custom theme overrides.
  - Narrow, comfortable, and full-width reading modes.
  - Minimum, default, and maximum zoom/font sizes.
- [ ] Fix known visual problem areas.
  - Text overlap or clipping.
  - Bad wrapping in tables, list items, inline code, and links.
  - Excessive or missing vertical spacing.
  - Low contrast in code blocks, line numbers, links, annotations, and blockquotes.
  - Image sizing that overflows or creates layout jumps.
- [ ] Add style-specific screenshot comparisons once fixtures exist.
  - Capture before/after screenshots for the styling fixture set.
  - Treat visible overlap, clipping, unreadable contrast, and broken wrapping as regressions.

### P2: Mermaid
- [ ] Revisit native Mermaid rendering.
  - Track upstream renderer compatibility.
  - Keep CLI fallback, but reduce dependency friction where possible.
  - Add clearer diagnostics when Node or `mmdc` is unavailable.
- [ ] Add Mermaid rendering fixtures.
  - Flowchart.
  - Sequence.
  - Class.
  - State.
  - Gantt.

### P2: Dependency & Build Hygiene
- [ ] Decide whether `Cargo.lock` should be tracked.
  - For application reproducibility, prefer tracking it.
  - If intentionally ignored, document the policy clearly.
- [ ] Add a dependency update workflow.
  - Run `cargo update`.
  - Run `cargo check --all-features`.
  - Run `cargo clippy --all-targets --all-features -- -D warnings`.
  - Run `cargo test --all-features`.
- [ ] Periodically review duplicate transitive dependencies.
  - Track `cargo tree -d`.
  - Avoid large migrations unless they reduce risk or improve performance.

### Security & Correctness (Codebase Review)
- [x] Annotation byte offset documentation aligned with pulldown-cmark
- [x] Annotation ID collision prevention (atomic counter)
- [x] Plugin sandbox hardened (removed require, loadstring, load, rawget, rawset, package)
- [x] Config save failure UI error feedback
- [x] Image path traversal prevention (canonicalization + base path check)
- [x] Annotation file size limits (10MB on load and save)
- [x] Network timeout for update checker (10s)
- [x] Theme switch cache invalidation (config hash + markdown cache reset)
- [x] Deterministic config hashing (FNV-1a replaces DefaultHasher)
- [x] File watcher canonicalized path comparison
- [x] Directory scan depth limit (32) and symlink cycle prevention
- [x] Windows file association exe path validation
- [x] Mermaid temp file collision-free naming (atomic counter)
- [x] Windows native menu null HWND safety check

### Performance (Codebase Review)
- [x] Mermaid metadata cache LRU eviction (max 200 entries)
- [x] Consolidated parse_hex_color to single implementation
- [x] TOC search results caching (invalidate on query change)
- [x] TOC visible indices mutation tracking (replaces hash-based invalidation)
- [x] Removed redundant annotation overlap check in AnnotationIndex
- [x] Removed redundant sync_file_watcher call (3x -> 2x per frame)
- [x] Syntax cache Arc<LayoutJob> to avoid clones on cache hits
- [x] Status bar file path cached (avoids per-frame allocation)
- [x] Magic numbers extracted to named constants
- [x] 3-char hex color shorthand support (#rgb)

## Known Limitations

- egui renders in-window menus; native menus are separate via muda
- Mermaid CLI requires Node.js + npm install -g @mermaid-js/mermaid-cli
- Native mermaid rendering disabled (upstream repo has Windows-incompatible paths)
- Full document structure processed every frame (partial viewport culling reduces expensive operations)
