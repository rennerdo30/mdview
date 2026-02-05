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

### Performance
- [x] Viewport culling (partial): skip expensive operations (image loading, syntax highlighting) for off-screen elements

### Features
- [x] Native menu bar: window-specific menu item states (enable/disable based on file state)

### Future Enhancements
- [x] Plugin: Lua API for menu customization (register_menu_item, unregister_menu_item)
- [x] Syntax highlighting in PDF export
- [x] Image support in PDF export (PNG, JPEG, BMP, GIF - local files only)
- [x] Loading indicator UI (spinner when is_loading is true)
- [x] Friendlier error messages (added friendly_errors module)

### Code Quality
- [x] Split render_menu_bar() (216 lines) - refactored into 6 focused functions

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
