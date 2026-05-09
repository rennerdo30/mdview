use std::path::Path;

use mdview::export::pdf::export_to_pdf_with_base;
use mdview::markdown::parser::parse_with_config;
use mdview::Config;
use tempfile::tempdir;

const FIXTURES: &[(&str, &str)] = &[
    ("dense", include_str!("../fixtures/markdown/dense.md")),
    (
        "typography",
        include_str!("../fixtures/markdown/typography.md"),
    ),
    (
        "edge-cases",
        include_str!("../fixtures/markdown/edge-cases.md"),
    ),
];

#[test]
fn markdown_style_fixtures_parse_successfully() {
    let config = Config::default();

    for (name, fixture) in FIXTURES {
        let events: Vec<_> = parse_with_config(fixture, &config).collect();
        assert!(!events.is_empty(), "{name} should produce markdown events");
    }
}

#[test]
fn dense_fixture_exports_pdf() {
    let config = Config::default();
    let fixture = include_str!("../fixtures/markdown/dense.md");
    let events: Vec<_> = parse_with_config(fixture, &config)
        .map(|event| event.into_static())
        .collect();

    let dir = tempdir().unwrap();
    let output = dir.path().join("dense.pdf");
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/markdown");

    export_to_pdf_with_base(&events, &output, &config, Some(&base)).unwrap();

    let bytes = std::fs::read(output).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.len() > 1_000);
}
