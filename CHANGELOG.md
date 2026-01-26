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
