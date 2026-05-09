# Mermaid Rendering

mdview renders Mermaid diagrams through the official `mmdc` CLI when it is available. Native Mermaid rendering remains disabled because the upstream Rust renderer still has portability issues for this app's supported platforms.

## Supported Path

Install:

```bash
npm install -g @mermaid-js/mermaid-cli
```

When `mmdc` is unavailable, mdview shows an in-app preview/fallback and an install hint instead of blocking document rendering.

## Fixtures

Mermaid fixtures live in `fixtures/mermaid`:

- `flowchart.mmd`
- `sequence.mmd`
- `class.mmd`
- `state.mmd`
- `gantt.mmd`

The renderer tests verify diagram type detection for this set. Full PNG rendering remains environment-dependent because it requires Node.js and `mmdc`.
