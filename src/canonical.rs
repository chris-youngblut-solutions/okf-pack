//! KC-5 — canonical serialization and lossless internal round-trips.
//!
//! Canonical form = `---` frontmatter (serde serializes [`Note`] fields in
//! declaration order: required → reserved → extension → links) followed by the
//! body verbatim. [`document_to_string`] and [`parse_document`] are exact
//! inverses for any canonical document; [`roundtrip_bundle`] is the loop gate.

use crate::models::Note;
use crate::surface::{Document, discover_md, split_frontmatter};
use anyhow::{Result, anyhow};
use std::path::Path;

/// Serialize a document to canonical `okf-ext` text.
pub fn document_to_string(doc: &Document) -> Result<String> {
    let mut frontmatter = serde_yml::to_string(&doc.note)?;
    if !frontmatter.ends_with('\n') {
        frontmatter.push('\n');
    }
    let body = doc.body.trim_end();
    Ok(if body.is_empty() {
        format!("---\n{frontmatter}---\n")
    } else {
        format!("---\n{frontmatter}---\n\n{body}\n")
    })
}

/// Parse a canonical document back into a [`Document`].
pub fn parse_document(text: &str) -> Result<Document> {
    let (frontmatter, body) = split_frontmatter(text);
    let frontmatter = frontmatter.ok_or_else(|| anyhow!("document has no frontmatter"))?;
    let note: Note = serde_yml::from_str(&frontmatter)?;
    Ok(Document { note, body })
}

/// Round-trip every note in a bundle through `parse → serialize → parse` and
/// report any that does not reproduce an equal model. Empty = lossless.
pub fn roundtrip_bundle(dir: &Path) -> Result<Vec<String>> {
    let mut errors = Vec::new();
    for file in discover_md(dir)? {
        let text = std::fs::read_to_string(&file)?;
        let once = match parse_document(&text) {
            Ok(doc) => doc,
            Err(e) => {
                errors.push(format!("{}: {e}", file.display()));
                continue;
            }
        };
        let reserialized = document_to_string(&once)?;
        match parse_document(&reserialized) {
            Ok(twice) if twice == once => {}
            Ok(_) => errors.push(format!(
                "{}: model changed across round-trip",
                file.display()
            )),
            Err(e) => errors.push(format!("{}: re-parse failed: {e}", file.display())),
        }
    }
    Ok(errors)
}
