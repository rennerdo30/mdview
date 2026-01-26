# mdview TODO

## Completed

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

- [ ] Native macOS menu bar: window-specific menu items (currently app-wide)
- [ ] Plugin: expand Lua API for menu customization
- [ ] Syntax highlighting in PDF export
- [ ] Image support in PDF export

## Known Limitations

- egui renders in-window menus; native menus are separate via muda
- Mermaid CLI requires Node.js + npm install -g @mermaid-js/mermaid-cli
