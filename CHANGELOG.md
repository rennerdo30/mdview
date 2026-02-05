# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of mdview
- Core markdown viewer with egui/eframe
- Table of Contents (TOC) sidebar with collapsible navigation
- Hot reload / file watching with debouncing
- Annotation system (highlights, notes, bookmarks)
- PDF export via printpdf
- Theme system with dark, light, sepia, and high-contrast themes
- TOML configuration support
- Syntax highlighting for code blocks (feature-gated)
- Lua plugin system (feature-gated)
- Drag and drop file support
- Keyboard shortcuts (Ctrl+O, Ctrl+T, Ctrl+P, F5)
- Cross-platform support (Windows, macOS, Linux)
- 3-character hex color shorthand support (#rgb expands to #rrggbb)

### Fixed
- Annotation byte offset documentation now correctly reflects pulldown-cmark byte offsets
- Annotation ID collisions prevented with atomic counter (timestamp + sequence)
- Plugin sandbox hardened: removed `require`, `loadstring`, `load`, `rawget`, `rawset`, `package`
- Config save failures now show UI error status instead of silently logging
- Theme switching now properly invalidates markdown cache (config hash + cache reset)
- Image path traversal prevented via canonicalization and base path containment check
- File watcher path comparison uses canonicalized paths for reliability
- Directory scanning limited to 32 levels deep to prevent stack overflow
- Symlinked directories skipped during scanning to prevent cycles
- Windows file association validates exe path against injection characters
- Mermaid temp files use atomic counter for collision-free filenames
- Windows native menu null HWND check prevents crash
- TOC keyboard navigation handles focused item outside visible list
- Multiple dropped files now logged instead of silently ignored
- Unhandled native menu events now produce log warnings
- Animation dt clamped to 0.1s to prevent overshoot on frame drops

### Changed
- Mermaid metadata cache now has LRU eviction (max 200 entries)
- Annotation storage enforces 10MB file size limit on both load and save
- Update checker uses 10-second network timeout
- Config hash uses deterministic FNV-1a (replaces non-deterministic DefaultHasher)
- Consolidated `parse_hex_color` to single canonical implementation in theme::style
- Syntax cache uses `Arc<LayoutJob>` to avoid expensive clones on cache hits
- Status bar file path cached to avoid per-frame allocation
- Magic numbers extracted to named constants (zoom limits, scroll factors, viewport buffer)
- TOC search results cached (invalidated only when query changes)
- TOC visible indices use direct mutation tracking instead of hash-based invalidation
- Redundant annotation overlap check removed from `AnnotationIndex::in_range()`
- Redundant `sync_file_watcher()` call removed (was called 3x per frame, now 2x)

### Technical
- Built with Rust and egui 0.29
- Markdown parsing via pulldown-cmark 0.12
- File watching via notify 7.0
- PDF generation via printpdf 0.7
- Optional syntax highlighting via syntect 5.2
- Optional Lua scripting via mlua 0.10

## [0.1.0] - 2025-01-26

### Added
- Initial project structure
- Basic markdown rendering
- TOC extraction and navigation
- File watcher integration
- Annotation data model and storage
- PDF export functionality
- Theme configuration
- CLI argument parsing

---

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| 0.1.0 | 2025-01-26 | Initial release |
