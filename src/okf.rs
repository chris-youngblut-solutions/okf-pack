//! KC-8 — export an internal `okf-ext` bundle to canonical OKF.
//!
//! The downgrade keeps the OKF-reserved projection (`type`/`title`/
//! `description`/`resource`/`tags`/`timestamp`) in the emitted note; everything
//! else (the closed-vocab extension fields, `x-*`, the structured links) goes to
//! a `.ckf/` sidecar so an internal round-trip stays lossless (KC-9). Typed links
//! also degrade to plain Markdown links under a generated `## Related` section so
//! the relationship survives for a pure-OKF reader. The privilege gate refuses
//! `--target okf-gcp` for any note matching the denylist.

use crate::canonical::parse_document;
use crate::models::{CkfType, LinkRel, Note};
use crate::privilege::Denylist;
use crate::surface::{
    Document, discover_md, file_stem, get_enum, get_str, get_vec_str, mtime_date,
    split_frontmatter, write_document,
};
use anyhow::{Result, anyhow, bail};
use serde::Serialize;
use serde_yml::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    FilesOnly,
    OkfGcp,
}

#[derive(Debug)]
pub struct ExportReport {
    pub item_count: usize,
    pub sha256: String,
    pub sidecar: bool,
}

/// The OKF-reserved projection of a note.
#[derive(Serialize)]
struct OkfNote {
    #[serde(rename = "type")]
    note_type: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    timestamp: String,
}

#[derive(Serialize)]
struct LinkRecord {
    rel: String,
    target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

/// Export an internal bundle to OKF at `dest`. Runs the privilege gate for
/// `Target::OkfGcp`. Returns a report (item count + sha256 over emitted notes).
pub fn export(
    bundle: &Path,
    dest: &Path,
    target: Target,
    denylist: &Denylist,
    sidecar: bool,
) -> Result<ExportReport> {
    let mut docs = Vec::new();
    for file in discover_md(bundle)? {
        let text = std::fs::read_to_string(&file)?;
        docs.push(parse_document(&text)?);
    }

    if target == Target::OkfGcp {
        let flagged: Vec<String> = docs
            .iter()
            .filter(|doc| {
                let combined = format!("{} {} {}", doc.note.id, doc.note.title, doc.body);
                !denylist.is_clean(&combined)
            })
            .map(|doc| doc.note.id.clone())
            .collect();
        if !flagged.is_empty() {
            bail!(
                "privilege gate: {} note(s) match the denylist — refusing --target okf-gcp ({}). \
                 Use --target files-only.",
                flagged.len(),
                flagged.join(", ")
            );
        }
    }

    std::fs::create_dir_all(dest)?;
    let ckf = dest.join(".ckf");
    if sidecar {
        std::fs::create_dir_all(ckf.join("sidecar"))?;
    }

    let mut hasher = Sha256::new();
    let mut links_json = serde_json::Map::new();
    let mut typemap = serde_json::Map::new();

    for doc in &docs {
        let okf_text = emit_okf_note(&doc.note, &doc.body)?;
        hasher.update(okf_text.as_bytes());
        let fname = sanitize_id(&doc.note.id);
        std::fs::write(dest.join(format!("{fname}.md")), &okf_text)?;

        let records = link_records(&doc.note);
        if !records.is_empty() {
            links_json.insert(doc.note.id.clone(), serde_json::to_value(&records)?);
        }
        typemap.insert(
            doc.note.id.clone(),
            serde_json::Value::String(type_str(&doc.note)),
        );

        if sidecar {
            let ext = extension_yaml(&doc.note)?;
            std::fs::write(ckf.join("sidecar").join(format!("{fname}.yaml")), ext)?;
        }
    }

    let sha = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if sidecar {
        std::fs::write(
            ckf.join("links.json"),
            serde_json::to_string_pretty(&links_json)?,
        )?;
        std::fs::write(
            ckf.join("typemap.json"),
            serde_json::to_string_pretty(&typemap)?,
        )?;
    }
    std::fs::write(
        dest.join("ckf.toml"),
        format!(
            "profile = \"okf-ext/0.1\"\nokf_conformance = \"0.1\"\nitem_count = {}\nsidecar = {}\nsha256 = \"{sha}\"\n",
            docs.len(),
            sidecar
        ),
    )?;

    Ok(ExportReport {
        item_count: docs.len(),
        sha256: sha,
        sidecar,
    })
}

fn emit_okf_note(note: &Note, body: &str) -> Result<String> {
    let okf = OkfNote {
        note_type: type_str(note),
        title: note.title.clone(),
        description: note.description.clone(),
        resource: note.resource.clone(),
        tags: note.tags.clone(),
        timestamp: note
            .timestamp
            .clone()
            .unwrap_or_else(|| note.updated.clone()),
    };
    let mut frontmatter = serde_yml::to_string(&okf)?;
    if !frontmatter.ends_with('\n') {
        frontmatter.push('\n');
    }

    let mut full_body = body.trim_end().to_string();
    if !note.links.is_empty() {
        full_body.push_str("\n\n## Related\n");
        for link in &note.links {
            let target = sanitize_id(&link.id);
            full_body.push_str(&format!(
                "- {}: [{}]({target}.md)\n",
                rel_str(link.rel),
                link.id
            ));
        }
    }

    Ok(if full_body.trim().is_empty() {
        format!("---\n{frontmatter}---\n")
    } else {
        format!("---\n{frontmatter}---\n\n{}\n", full_body.trim_end())
    })
}

fn link_records(note: &Note) -> Vec<LinkRecord> {
    note.links
        .iter()
        .map(|l| LinkRecord {
            rel: rel_str(l.rel),
            target: l.id.clone(),
            note: l.note.clone(),
        })
        .collect()
}

/// The extension layer (everything not in the OKF projection) as YAML.
fn extension_yaml(note: &Note) -> Result<String> {
    let mut value = serde_yml::to_value(note)?;
    if let Some(map) = value.as_mapping_mut() {
        for key in ["type", "title", "description", "resource", "tags"] {
            map.remove(key);
        }
    }
    Ok(serde_yml::to_string(&value)?)
}

fn type_str(note: &Note) -> String {
    serde_json::to_value(note.note_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn rel_str(rel: LinkRel) -> String {
    serde_json::to_value(rel)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn sanitize_id(id: &str) -> String {
    id.replace(['/', ' ', '\\'], "-")
}

// --------------------------------------------------------------------------
// import (OKF -> internal okf-ext)
// --------------------------------------------------------------------------

/// Import an OKF bundle into the internal `okf-ext` form at `dest`.
///
/// If the bundle carries a `.ckf/sidecar/` (one we exported), reconstruction is
/// lossless: the OKF projection is merged with the per-note extension sidecar and
/// the generated `## Related` section is stripped. A foreign bundle (no sidecar)
/// lands loosely (type/title/description/resource/tags/timestamp + body verbatim).
pub fn import(okf_bundle: &Path, dest: &Path) -> Result<usize> {
    let sidecar_dir = okf_bundle.join(".ckf").join("sidecar");
    let has_sidecar = sidecar_dir.is_dir();
    std::fs::create_dir_all(dest)?;

    let mut count = 0;
    for file in discover_md(okf_bundle)? {
        let text = std::fs::read_to_string(&file)?;
        let (Some(frontmatter), body) = split_frontmatter(&text) else {
            continue;
        };
        let okf_fm: Value = serde_yml::from_str(&frontmatter)?;
        let stem = file_stem(&file);
        let sidecar = sidecar_dir.join(format!("{stem}.yaml"));

        let note = if has_sidecar && sidecar.exists() {
            reconstruct_from_sidecar(&okf_fm, &sidecar)?
        } else {
            loose_okf_note(&okf_fm, &stem, &file)
        };

        let body = if note.links.is_empty() {
            body
        } else {
            strip_related(&body)
        };
        write_document(dest, &Document { note, body })?;
        count += 1;
    }
    Ok(count)
}

/// Merge the OKF projection with the extension sidecar into a full internal note.
fn reconstruct_from_sidecar(okf_fm: &Value, sidecar: &Path) -> Result<Note> {
    let mut ext: Value = serde_yml::from_str(&std::fs::read_to_string(sidecar)?)?;
    let map = ext
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("sidecar is not a mapping"))?;
    if let Some(src) = okf_fm.as_mapping() {
        for key in ["type", "title", "description", "resource", "tags"] {
            if let Some(v) = src.get(key) {
                map.insert(key, v.clone());
            }
        }
        // `updated` is the internal canonical date; fall back to OKF `timestamp`.
        if map.get("updated").is_none()
            && let Some(ts) = src.get("timestamp")
        {
            map.insert("updated", ts.clone());
        }
    }
    Ok(serde_yml::from_value(ext)?)
}

/// Loose mapping for a foreign OKF bundle (no sidecar): id from filename.
fn loose_okf_note(fm: &Value, stem: &str, file: &Path) -> Note {
    Note {
        note_type: get_enum::<CkfType>(fm, "type").unwrap_or(CkfType::Doc),
        id: stem.to_string(),
        title: get_str(fm, "title").unwrap_or_else(|| stem.to_string()),
        updated: get_str(fm, "timestamp")
            .or_else(|| get_str(fm, "updated"))
            .unwrap_or_else(|| mtime_date(file)),
        description: get_str(fm, "description"),
        resource: get_str(fm, "resource"),
        tags: get_vec_str(fm, "tags"),
        timestamp: get_str(fm, "timestamp"),
        ..Note::stub()
    }
}

/// Remove a trailing generated `## Related` section (the last occurrence).
fn strip_related(body: &str) -> String {
    match body.rfind("## Related") {
        Some(idx) => body[..idx].trim_end().to_string(),
        None => body.trim_end().to_string(),
    }
}
