//! KC-9 — bundle validation with rule-tagged findings.
//!
//! Rules:
//! - `E001-no-frontmatter` — a `.md` with no YAML frontmatter.
//! - `E002-schema` — frontmatter that does not deserialize into a [`Note`]
//!   (unknown field, missing required field, out-of-vocabulary enum).
//! - `E003-dangling-link` — a non-`mentions` typed link whose target id is not
//!   present elsewhere in the bundle.
//! - `E004-index-type` — an `index.md` whose `type` is not `index`.

use crate::models::{CkfType, LinkRel, Note};
use crate::surface::{discover_md, file_stem, split_frontmatter};
use anyhow::Result;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: String,
    pub rule: &'static str,
    pub message: String,
}

/// Validate every note in a bundle. Empty result = valid.
pub fn validate_bundle(dir: &Path) -> Result<Vec<Finding>> {
    let files = discover_md(dir)?;

    // First pass: parse notes and collect ids for the cross-link check.
    let mut parsed: Vec<(std::path::PathBuf, Option<String>, Option<Note>)> = Vec::new();
    let mut ids = BTreeSet::new();
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        let frontmatter = split_frontmatter(&text).0;
        let note = frontmatter
            .as_deref()
            .and_then(|f| serde_yml::from_str::<Note>(f).ok());
        if let Some(n) = &note {
            ids.insert(n.id.clone());
        }
        parsed.push((file.clone(), frontmatter, note));
    }

    let mut findings = Vec::new();
    for (file, frontmatter, note) in &parsed {
        let path = file.display().to_string();
        match (frontmatter, note) {
            (None, _) => findings.push(Finding {
                path,
                rule: "E001-no-frontmatter",
                message: "missing YAML frontmatter".into(),
            }),
            (Some(raw), None) => {
                let message = serde_yml::from_str::<Note>(raw)
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_default();
                findings.push(Finding {
                    path,
                    rule: "E002-schema",
                    message,
                });
            }
            (Some(_), Some(note)) => {
                for link in &note.links {
                    if link.rel != LinkRel::Mentions && !ids.contains(&link.id) {
                        findings.push(Finding {
                            path: path.clone(),
                            rule: "E003-dangling-link",
                            message: format!(
                                "typed link target `{}` is not in the bundle",
                                link.id
                            ),
                        });
                    }
                }
                if file_stem(file) == "index" && note.note_type != CkfType::Index {
                    findings.push(Finding {
                        path: path.clone(),
                        rule: "E004-index-type",
                        message: "index.md must declare `type: index`".into(),
                    });
                }
            }
        }
    }
    Ok(findings)
}
