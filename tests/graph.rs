//! KC-11 — graph build + json/html emit.

use okf_pack::graph::{build, to_html, to_json};
use std::path::{Path, PathBuf};

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

#[test]
fn graph_builds_nodes_and_typed_edges() {
    let graph = build(&fixture("sample-space")).unwrap();
    assert_eq!(graph.nodes.len(), 3, "three notes");
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.source == "okf-spec" && e.target == "concept-b" && e.rel == "related"),
        "the typed related edge is present: {:?}",
        graph.edges
    );
}

#[test]
fn graph_emits_json_and_self_contained_html() {
    let graph = build(&fixture("sample-space")).unwrap();

    let json = to_json(&graph).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["nodes"].as_array().unwrap().len(), 3);

    let html = to_html(&graph).unwrap();
    assert!(!html.contains("__GRAPH_DATA__"), "placeholder substituted");
    assert!(html.contains("<svg"), "inline SVG");
    assert!(html.contains("Open Knowledge Format"), "node data embedded");
}
