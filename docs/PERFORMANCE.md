# Performance Regression Workflow

mdview keeps the normal test suite deterministic and fast. Larger performance checks live in ignored tests so they can be run on a quiet local machine before risky parser, renderer, search, or dependency changes.

## Run Benchmarks

```bash
cargo test --test performance_regression -- --ignored --nocapture
```

The harness generates stress documents at runtime:

- 1k-line plain Markdown.
- 5k-line heading-heavy Markdown.
- 10k-line mixed Markdown with tables, code fences, tasks, blockquotes, footnotes, links, and repeated searchable text.

It currently checks:

- Initial parse plus TOC construction.
- Full-document search index construction.
- Markdown event cache reuse.

Thresholds are intentionally loose. They are meant to catch obvious regressions, not microbenchmark noise. If a threshold fails, rerun on a quiet machine before changing it.

## Per-Frame Audit Notes

Known hot paths and current mitigations:

- Parsing: cached in `AppState::get_cached_events`.
- TOC generation: reuses parsed events when files load.
- Search: cached by content hash plus query and not rebuilt every frame.
- Syntax highlighting: LRU cache stores `Arc<LayoutJob>` values.
- Images: async load plus LRU texture cache.
- Mermaid: async render plus metadata/render result caches.
- Renderer culling: block-level viewport culling skips expensive off-screen work while forcing headings, footnote definitions, and active navigation targets to render.
- Annotation lookup: sorted index plus binary search avoids scanning every annotation for every text run.

Manual profiling should focus on:

- Very wide tables.
- Long code blocks with syntax highlighting and line numbers.
- Documents with thousands of headings.
- Documents with many overlapping annotations.
- Plugin hooks when the `plugins` feature is enabled.
