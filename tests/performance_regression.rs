use std::sync::Arc;
use std::time::{Duration, Instant};

use mdview::app::state::AppState;
use mdview::markdown::parser::parse_with_config;
use mdview::toc::builder::build_toc_from_events;
use mdview::Config;
use sha2::{Digest, Sha256};

fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_plain_document(lines: usize) -> String {
    let mut doc = String::with_capacity(lines * 96);
    doc.push_str("# Large Plain Document\n\n");
    for i in 0..lines {
        doc.push_str(&format!(
            "Paragraph line {} with enough prose to exercise wrapping, parsing, and search behavior.\n\n",
            i
        ));
    }
    doc
}

fn generate_heading_heavy_document(lines: usize) -> String {
    let mut doc = String::with_capacity(lines * 88);
    for i in 0..lines {
        let level = (i % 6) + 1;
        doc.push_str(&format!(
            "{} Section {}\n\nBody text for section {} with repeated navigation anchors.\n\n",
            "#".repeat(level),
            i,
            i
        ));
    }
    doc
}

fn generate_mixed_heavy_document(lines: usize) -> String {
    let mut doc = String::with_capacity(lines * 150);
    doc.push_str("# Mixed Stress Document\n\n");
    for i in 0..lines {
        match i % 5 {
            0 => doc.push_str(&format!(
                "## Heading {}\n\nParagraph with **bold**, *italic*, `inline code`, and [a link](https://example.com/{}).\n\n",
                i, i
            )),
            1 => doc.push_str(&format!(
                "| Key | Value |\n| --- | ----- |\n| row-{} | {} |\n\n",
                i,
                "wide cell content ".repeat(8)
            )),
            2 => doc.push_str(&format!(
                "```rust\nfn case_{}() {{ println!(\"stress\"); }}\n```\n\n",
                i
            )),
            3 => doc.push_str(&format!(
                "- [x] completed item {}\n- [ ] pending item {}\n\n",
                i, i
            )),
            _ => doc.push_str(&format!(
                "> Quote block {} with a footnote reference.[^note{}]\n\n[^note{}]: Footnote body.\n\n",
                i, i, i
            )),
        }
    }
    doc
}

fn parse_and_build_toc(content: &str, config: &Config) -> Duration {
    let started = Instant::now();
    let events: Vec<_> = parse_with_config(content, config)
        .map(|event| event.into_static())
        .collect();
    let _toc = build_toc_from_events(events.iter());
    started.elapsed()
}

#[test]
#[ignore = "performance regression harness; run manually on a quiet machine"]
fn benchmark_initial_load_fixtures() {
    let config = Config::default();
    let fixtures = [
        (
            "plain-1k",
            generate_plain_document(1_000),
            Duration::from_millis(250),
        ),
        (
            "headings-5k",
            generate_heading_heavy_document(5_000),
            Duration::from_millis(900),
        ),
        (
            "mixed-10k",
            generate_mixed_heavy_document(10_000),
            Duration::from_millis(2_500),
        ),
    ];

    for (name, content, threshold) in fixtures {
        let elapsed = parse_and_build_toc(&content, &config);
        assert!(
            elapsed <= threshold,
            "{name} parse+toc took {:?}, threshold {:?}",
            elapsed,
            threshold
        );
    }
}

#[test]
#[ignore = "performance regression harness; run manually on a quiet machine"]
fn benchmark_search_index_large_document() {
    let content = generate_mixed_heavy_document(10_000);
    let mut state = AppState::new(Config::default());
    state.content_hash = compute_content_hash(&content);
    state.content = Arc::new(content);

    let started = Instant::now();
    state.set_document_search_query("stress".to_string());
    let elapsed = started.elapsed();

    assert!(!state.document_search.matches.is_empty());
    assert!(
        elapsed <= Duration::from_millis(200),
        "search indexing took {:?}",
        elapsed
    );
}

#[test]
#[ignore = "performance regression harness; run manually on a quiet machine"]
fn benchmark_markdown_cache_reuse() {
    let mut state = AppState::new(Config::default());
    let content = generate_mixed_heavy_document(5_000);
    state.content_hash = compute_content_hash(&content);
    state.content = Arc::new(content);

    let cold_start = Instant::now();
    let cold_events = state.get_cached_events();
    let cold_elapsed = cold_start.elapsed();

    let warm_start = Instant::now();
    let warm_events = state.get_cached_events();
    let warm_elapsed = warm_start.elapsed();

    assert_eq!(cold_events.len(), warm_events.len());
    assert!(warm_elapsed < cold_elapsed / 10 || warm_elapsed <= Duration::from_millis(2));
}
