//! KC-11 — build and emit the link graph.
//!
//! Nodes are notes; edges are their typed links. Emits `json` (always) and a
//! self-contained, zero-dependency `html` viz (inline SVG, circular layout) for
//! client hand-offs. The `kuzu` backend was dropped — see ADR 0001.

use crate::canonical::parse_document;
use crate::models::{CkfType, LinkRel, Note};
use crate::surface::discover_md;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub note_type: String,
    pub title: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Edge {
    pub source: String,
    pub rel: String,
    pub target: String,
}

#[derive(Debug, Serialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Build the link graph from every note in a bundle.
pub fn build(dir: &Path) -> Result<Graph> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for file in discover_md(dir)? {
        let text = std::fs::read_to_string(&file)?;
        let Ok(doc) = parse_document(&text) else {
            continue;
        };
        let note = &doc.note;
        for link in &note.links {
            edges.push(Edge {
                source: note.id.clone(),
                rel: rel_str(link.rel),
                target: link.id.clone(),
            });
        }
        nodes.push(Node {
            id: note.id.clone(),
            note_type: type_str(note),
            title: note.title.clone(),
        });
    }
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    edges.sort_by(|a, b| (&a.source, &a.target, &a.rel).cmp(&(&b.source, &b.target, &b.rel)));
    Ok(Graph { nodes, edges })
}

/// Serialize the graph as pretty JSON.
pub fn to_json(graph: &Graph) -> Result<String> {
    Ok(serde_json::to_string_pretty(graph)?)
}

/// Render a self-contained HTML viz (inline SVG, no external dependencies).
pub fn to_html(graph: &Graph) -> Result<String> {
    let data = serde_json::to_string(graph)?;
    Ok(HTML_TEMPLATE.replace("__GRAPH_DATA__", &data))
}

fn type_str(note: &Note) -> String {
    enum_str(note.note_type)
}

fn rel_str(rel: LinkRel) -> String {
    serde_json::to_value(rel)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn enum_str(t: CkfType) -> String {
    serde_json::to_value(t)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

const HTML_TEMPLATE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>okf-pack graph</title>
<style>body{font:13px sans-serif;margin:0}svg{width:100vw;height:100vh}
circle{fill:#4a90d9}text{fill:#222}line{stroke:#aaa}</style></head>
<body><svg id="g"></svg>
<script>
const G = __GRAPH_DATA__;
const svg = document.getElementById('g');
const W = window.innerWidth, H = window.innerHeight, R = Math.min(W, H) / 2.5;
const cx = W / 2, cy = H / 2, n = G.nodes.length || 1, pos = {};
G.nodes.forEach((nd, i) => { const a = 2 * Math.PI * i / n; pos[nd.id] = [cx + R * Math.cos(a), cy + R * Math.sin(a)]; });
const NS = 'http://www.w3.org/2000/svg';
G.edges.forEach(e => { const s = pos[e.source], t = pos[e.target]; if (!s || !t) return;
  const l = document.createElementNS(NS, 'line');
  l.setAttribute('x1', s[0]); l.setAttribute('y1', s[1]); l.setAttribute('x2', t[0]); l.setAttribute('y2', t[1]);
  svg.appendChild(l); });
G.nodes.forEach(nd => { const p = pos[nd.id];
  const c = document.createElementNS(NS, 'circle'); c.setAttribute('cx', p[0]); c.setAttribute('cy', p[1]); c.setAttribute('r', 8); svg.appendChild(c);
  const tx = document.createElementNS(NS, 'text'); tx.setAttribute('x', p[0] + 11); tx.setAttribute('y', p[1] + 4); tx.textContent = nd.title || nd.id; svg.appendChild(tx); });
</script></body></html>
"#;
