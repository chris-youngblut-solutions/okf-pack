//! KC-4 — surface adapters.
//!
//! A [`SurfaceAdapter`] reads a live operator knowledge surface into the unified
//! [`Document`] (an [`Note`] + its Markdown body). Built-ins cover the five
//! surfaces: `memory` (bimodal flat/nested frontmatter), `skills`, `devtel`
//! (frozen casing), `container` (synthesizes frontmatter from a prose header),
//! and `okf` (the interchange surface). [`migrate`] walks a surface and writes
//! an `okf-ext` bundle; [`validate_bundle`] is the minimal schema gate (KC-9
//! enriches it).

use crate::models::{CkfType, LinkRel, MemKind, Note, Status, TypedLink};
use crate::parse;
use anyhow::{Context, Result, anyhow, bail};
use serde::de::DeserializeOwned;
use serde_yml::Value;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// A parsed note plus its Markdown body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub note: Note,
    pub body: String,
}

/// Reads one knowledge surface into [`Document`]s.
pub trait SurfaceAdapter {
    fn name(&self) -> &'static str;
    /// dev.tel preserves capitalized note-type folders — never lowercase its paths.
    fn frozen_casing(&self) -> bool {
        false
    }
    fn discover(&self, root: &Path) -> Result<Vec<PathBuf>>;
    fn read_note(&self, path: &Path, root: &Path) -> Result<Document>;
}

/// Resolve a surface adapter by name.
pub fn adapter_for(name: &str) -> Result<Box<dyn SurfaceAdapter>> {
    Ok(match name {
        "memory" => Box::new(MemorySurface),
        "skills" => Box::new(SkillsSurface),
        "devtel" => Box::new(DevtelSurface),
        "container" => Box::new(ContainerSurface),
        "okf" => Box::new(OkfSurface),
        other => bail!("unknown surface: {other} (memory|skills|devtel|container|okf)"),
    })
}

/// Walk a surface and write each note as an `okf-ext` bundle file under `dest`.
/// Returns the number of notes written.
pub fn migrate(surface: &str, src: &Path, dest: &Path) -> Result<usize> {
    let adapter = adapter_for(surface)?;
    let files = adapter.discover(src)?;
    std::fs::create_dir_all(dest)?;
    let mut count = 0;
    for file in files {
        let doc = adapter
            .read_note(&file, src)
            .with_context(|| format!("reading {} via {} surface", file.display(), surface))?;
        write_document(dest, &doc)?;
        count += 1;
    }
    Ok(count)
}

/// Write one document as `<dest>/<id>.md` in canonical form.
pub fn write_document(dest: &Path, doc: &Document) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let fname = sanitize_id(&doc.note.id);
    std::fs::write(
        dest.join(format!("{fname}.md")),
        crate::canonical::document_to_string(doc)?,
    )?;
    Ok(())
}

/// Validate a bundle, returning human-readable, rule-tagged error strings
/// (empty = valid). Delegates to [`crate::validate`].
pub fn validate_bundle(dir: &Path) -> Result<Vec<String>> {
    Ok(crate::validate::validate_bundle(dir)?
        .into_iter()
        .map(|f| format!("{}: [{}] {}", f.path, f.rule, f.message))
        .collect())
}

fn sanitize_id(id: &str) -> String {
    id.replace(['/', ' ', '\\'], "-")
}

// --------------------------------------------------------------------------
// shared helpers
// --------------------------------------------------------------------------

pub(crate) fn discover_md(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_md(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_md(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if is_md(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in std::fs::read_dir(path)? {
        let p = entry?.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if p.is_dir() {
            walk_md(&p, out)?;
        } else if is_md(&p) {
            out.push(p);
        }
    }
    Ok(())
}

fn is_md(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("md")
}

pub(crate) fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// Split a `---`-delimited YAML frontmatter block from the body.
pub(crate) fn split_frontmatter(text: &str) -> (Option<String>, String) {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return (None, text.to_string());
    }
    let mut frontmatter = String::new();
    let mut body: Vec<&str> = Vec::new();
    let mut closed = false;
    for line in lines {
        if !closed && line.trim_end() == "---" {
            closed = true;
            continue;
        }
        if closed {
            body.push(line);
        } else {
            frontmatter.push_str(line);
            frontmatter.push('\n');
        }
    }
    if !closed {
        return (None, text.to_string());
    }
    (
        Some(frontmatter),
        body.join("\n").trim_start_matches('\n').to_string(),
    )
}

pub(crate) fn get_str(v: &Value, key: &str) -> Option<String> {
    v.as_mapping()?.get(key)?.as_str().map(str::to_string)
}

pub(crate) fn get_vec_str(v: &Value, key: &str) -> Vec<String> {
    v.as_mapping()
        .and_then(|m| m.get(key))
        .and_then(Value::as_sequence)
        .map(|s| {
            s.iter()
                .filter_map(|e| e.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn get_enum<T: DeserializeOwned + 'static>(v: &Value, key: &str) -> Option<T> {
    let val = v.as_mapping()?.get(key)?.clone();
    serde_yml::from_value(val).ok()
}

/// Tolerant frontmatter reader for foreign operator surfaces.
///
/// Real Claude-Code memory/skill frontmatter routinely carries unquoted colons
/// in scalar values (`description: …it's live: resume…`), which strict YAML
/// rejects with "mapping values are not allowed in this context". This reader
/// tries strict YAML first (preserving block scalars, typed lists, exact types)
/// and only falls back to a forgiving line parser when strict parsing fails. The
/// fallback treats each `key: value` line as a string assignment (the value is
/// the rest of the line, verbatim), supports one level of indented nesting
/// (`metadata:` blocks) and `- ` / `[a, b]` lists, strips matched surrounding
/// quotes, and never errors. It is used only for surfaces we don't author; our
/// own emitted okf-ext notes are parsed strictly.
pub(crate) fn parse_fm_tolerant(fm: &str) -> Value {
    serde_yml::from_str::<Value>(fm).unwrap_or_else(|_| lenient_frontmatter(fm))
}

fn lenient_frontmatter(fm: &str) -> Value {
    let lines: Vec<(usize, &str)> = fm
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| (l.len() - l.trim_start().len(), l))
        .collect();
    let mut pos = 0;
    let base = lines.first().map_or(0, |(i, _)| *i);
    lenient_map(&lines, &mut pos, base)
}

fn lenient_map(lines: &[(usize, &str)], pos: &mut usize, base: usize) -> Value {
    let mut map = serde_yml::Mapping::new();
    while *pos < lines.len() {
        let (indent, raw) = lines[*pos];
        if indent < base {
            break;
        }
        if indent > base {
            // over-indented with no owning key — skip defensively
            *pos += 1;
            continue;
        }
        let content = raw.trim_start();
        let Some((key, after)) = content.split_once(':') else {
            *pos += 1;
            continue;
        };
        let key = key.trim().to_string();
        let after = after.trim();
        *pos += 1;
        if after.is_empty() {
            // a nested block or list — decided by the next line's indent + shape
            if *pos < lines.len() && lines[*pos].0 > base {
                let child = lines[*pos].0;
                if lines[*pos].1.trim_start().starts_with("- ") {
                    map.insert(key, Value::Sequence(lenient_list(lines, pos, child)));
                } else {
                    map.insert(key, lenient_map(lines, pos, child));
                }
            } else {
                map.insert(key, Value::String(String::new()));
            }
        } else {
            map.insert(key, lenient_scalar(after));
        }
    }
    Value::Mapping(map)
}

fn lenient_list(lines: &[(usize, &str)], pos: &mut usize, base: usize) -> Vec<Value> {
    let mut out = Vec::new();
    while *pos < lines.len() {
        let (indent, raw) = lines[*pos];
        match (indent == base)
            .then(|| raw.trim_start().strip_prefix("- "))
            .flatten()
        {
            Some(item) => {
                out.push(lenient_scalar(item.trim()));
                *pos += 1;
            }
            None => break,
        }
    }
    out
}

fn lenient_scalar(s: &str) -> Value {
    if let Some(inner) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        let items = inner
            .split(',')
            .map(|p| unquote(p.trim()))
            .filter(|p| !p.is_empty())
            .map(|p| Value::String(p.to_string()))
            .collect();
        return Value::Sequence(items);
    }
    Value::String(unquote(s).to_string())
}

fn unquote(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 2
        && ((b[0] == b'"' && b[b.len() - 1] == b'"') || (b[0] == b'\'' && b[b.len() - 1] == b'\''))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

pub(crate) fn mtime_date(path: &Path) -> String {
    let dt: time::OffsetDateTime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(time::OffsetDateTime::from)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!(
        "{:04}-{:02}-{:02}",
        dt.year(),
        u8::from(dt.month()),
        dt.day()
    )
}

/// `updated` from frontmatter (`updated`/`created`/`timestamp`) or file mtime.
fn updated_or_mtime(fm: Option<&Value>, path: &Path) -> String {
    fm.and_then(|v| {
        get_str(v, "updated")
            .or_else(|| get_str(v, "created"))
            .or_else(|| get_str(v, "timestamp"))
    })
    .unwrap_or_else(|| mtime_date(path))
}

fn mentions_from_body(body: &str) -> Vec<TypedLink> {
    parse::body_links(body)
        .wikilinks
        .into_iter()
        .map(|id| TypedLink {
            id,
            rel: LinkRel::Mentions,
            note: None,
        })
        .collect()
}

fn typed_links(ids: Vec<String>, rel: LinkRel) -> impl Iterator<Item = TypedLink> {
    ids.into_iter().map(move |id| TypedLink {
        id,
        rel,
        note: None,
    })
}

// --------------------------------------------------------------------------
// memory
// --------------------------------------------------------------------------

struct MemorySurface;

impl SurfaceAdapter for MemorySurface {
    fn name(&self) -> &'static str {
        "memory"
    }
    fn discover(&self, root: &Path) -> Result<Vec<PathBuf>> {
        discover_md(root)
    }
    fn read_note(&self, path: &Path, _root: &Path) -> Result<Document> {
        let text = std::fs::read_to_string(path)?;
        let (fm_raw, body) = split_frontmatter(&text);

        // MEMORY.md is the directory index.
        if path.file_name().and_then(|s| s.to_str()) == Some("MEMORY.md") {
            let note = Note {
                note_type: CkfType::Index,
                id: "index".into(),
                title: "Memory index".into(),
                updated: mtime_date(path),
                ..Note::stub()
            };
            return Ok(Document { note, body });
        }

        let fm: Value = fm_raw
            .as_deref()
            .map(parse_fm_tolerant)
            .unwrap_or(Value::Null);

        // bimodal: `type`/`originSessionId` at top level OR under `metadata`.
        let meta = fm.as_mapping().and_then(|m| m.get("metadata"));
        let kind_str = get_str(&fm, "type").or_else(|| meta.and_then(|m| get_str(m, "type")));
        let x_mem_kind =
            kind_str.and_then(|s| serde_yml::from_value::<MemKind>(Value::String(s)).ok());
        let origin = get_str(&fm, "originSessionId")
            .or_else(|| meta.and_then(|m| get_str(m, "originSessionId")));
        let name = get_str(&fm, "name").unwrap_or_else(|| file_stem(path));

        let note = Note {
            note_type: CkfType::Memory,
            id: name.clone(),
            title: name,
            updated: updated_or_mtime(Some(&fm), path),
            description: get_str(&fm, "description"),
            x_mem_kind,
            x_origin_session_id: origin,
            links: mentions_from_body(&body),
            ..Note::stub()
        };
        Ok(Document { note, body })
    }
}

// --------------------------------------------------------------------------
// skills
// --------------------------------------------------------------------------

struct SkillsSurface;

impl SurfaceAdapter for SkillsSurface {
    fn name(&self) -> &'static str {
        "skills"
    }
    fn discover(&self, root: &Path) -> Result<Vec<PathBuf>> {
        Ok(discover_md(root)?
            .into_iter()
            .filter(|p| p.file_name().and_then(|s| s.to_str()) == Some("SKILL.md"))
            .collect())
    }
    fn read_note(&self, path: &Path, _root: &Path) -> Result<Document> {
        let text = std::fs::read_to_string(path)?;
        let (fm_raw, body) = split_frontmatter(&text);
        let fm: Value = fm_raw
            .as_deref()
            .map(parse_fm_tolerant)
            .unwrap_or(Value::Null);
        let name = get_str(&fm, "name").unwrap_or_else(|| file_stem(path));
        let note = Note {
            note_type: CkfType::Skill, // inferred (skills carry no `type`)
            id: name.clone(),
            title: name,
            updated: updated_or_mtime(Some(&fm), path),
            description: get_str(&fm, "description"),
            x_when_to_use: get_str(&fm, "when_to_use"),
            x_argument_hint: get_str(&fm, "argument-hint"),
            x_allowed_tools: get_str(&fm, "allowed-tools"),
            ..Note::stub()
        };
        Ok(Document { note, body })
    }
}

// --------------------------------------------------------------------------
// devtel (frozen casing)
// --------------------------------------------------------------------------

struct DevtelSurface;

impl SurfaceAdapter for DevtelSurface {
    fn name(&self) -> &'static str {
        "devtel"
    }
    fn frozen_casing(&self) -> bool {
        true
    }
    fn discover(&self, root: &Path) -> Result<Vec<PathBuf>> {
        discover_md(root)
    }
    fn read_note(&self, path: &Path, _root: &Path) -> Result<Document> {
        let text = std::fs::read_to_string(path)?;
        let (fm_raw, body) = split_frontmatter(&text);
        let fm: Value = fm_raw
            .as_deref()
            .map(parse_fm_tolerant)
            .unwrap_or(Value::Null);

        let mut links: Vec<TypedLink> = Vec::new();
        links.extend(typed_links(get_vec_str(&fm, "related"), LinkRel::Related));
        links.extend(typed_links(
            get_vec_str(&fm, "depends-on"),
            LinkRel::DependsOn,
        ));
        links.extend(mentions_from_body(&body));

        let note = Note {
            note_type: get_enum::<CkfType>(&fm, "type").unwrap_or(CkfType::Doc),
            id: get_str(&fm, "id").unwrap_or_else(|| file_stem(path)),
            title: get_str(&fm, "title").unwrap_or_else(|| file_stem(path)),
            updated: updated_or_mtime(Some(&fm), path),
            created: Some(updated_or_mtime(Some(&fm), path)),
            description: get_str(&fm, "description"),
            root: get_enum(&fm, "root"),
            tier: get_enum(&fm, "tier"),
            status: get_enum(&fm, "status"),
            fde_domain: get_enum::<Vec<_>>(&fm, "fde-domain").unwrap_or_default(),
            container: get_str(&fm, "container"),
            container_path: get_str(&fm, "container-path"),
            tech: get_vec_str(&fm, "tech"),
            hardware: get_vec_str(&fm, "hardware"),
            stakeholder: get_vec_str(&fm, "stakeholder"),
            supersedes: get_str(&fm, "supersedes"),
            superseded_by: get_str(&fm, "superseded-by"),
            links,
            ..Note::stub()
        };
        Ok(Document { note, body })
    }
}

// --------------------------------------------------------------------------
// container (prose-header → frontmatter)
// --------------------------------------------------------------------------

struct ContainerSurface;

fn bold_meta_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\*\*([^*:]+):\*\*\s*([^*\n]+)").unwrap())
}

fn parse_container_status(s: &str) -> Option<Status> {
    match s.to_lowercase().as_str() {
        "accepted" | "adopted" | "active" => Some(Status::Active),
        "superseded" => Some(Status::Superseded),
        "reversed" | "rejected" => Some(Status::Reversed),
        other => serde_yml::from_value::<Status>(Value::String(other.to_string())).ok(),
    }
}

impl SurfaceAdapter for ContainerSurface {
    fn name(&self) -> &'static str {
        "container"
    }
    fn discover(&self, root: &Path) -> Result<Vec<PathBuf>> {
        discover_md(root)
    }
    fn read_note(&self, path: &Path, _root: &Path) -> Result<Document> {
        // No frontmatter — the prose IS the source; the body is kept verbatim.
        let body = std::fs::read_to_string(path)?;

        let note_type = match path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
        {
            Some("adr") => CkfType::Adr,
            Some("scope") => CkfType::Scope,
            Some("tasks") => CkfType::Task,
            _ => CkfType::Doc,
        };

        let title = body
            .lines()
            .find_map(|l| l.strip_prefix("# "))
            .map(|t| t.trim().to_string())
            .unwrap_or_else(|| file_stem(path));

        let (mut status, mut created, mut container, mut supersedes) = (None, None, None, None);
        for caps in bold_meta_re().captures_iter(&body) {
            let label = caps[1].trim().to_lowercase();
            let value = caps[2].trim().trim_end_matches('·').trim().to_string();
            if label.starts_with("status") {
                status = parse_container_status(&value);
            } else if label.starts_with("date") {
                created = Some(value);
            } else if label.starts_with("container") {
                container = Some(value);
            } else if label.starts_with("supersede") {
                supersedes = Some(value);
            }
        }

        // id = <container>-<nnnn-from-filename> (fall back to the file stem).
        let stem = file_stem(path);
        let seq = stem
            .split('-')
            .next()
            .filter(|s| s.chars().all(|c| c.is_ascii_digit()));
        let id = match (&container, seq) {
            (Some(c), Some(n)) => format!("{c}-{n}"),
            _ => stem,
        };

        let note = Note {
            note_type,
            id,
            title,
            updated: created.clone().unwrap_or_else(|| mtime_date(path)),
            created,
            status,
            container,
            supersedes,
            ..Note::stub()
        };
        Ok(Document { note, body })
    }
}

// --------------------------------------------------------------------------
// okf (interchange)
// --------------------------------------------------------------------------

struct OkfSurface;

impl SurfaceAdapter for OkfSurface {
    fn name(&self) -> &'static str {
        "okf"
    }
    fn discover(&self, root: &Path) -> Result<Vec<PathBuf>> {
        discover_md(root)
    }
    fn read_note(&self, path: &Path, _root: &Path) -> Result<Document> {
        let text = std::fs::read_to_string(path)?;
        let (fm_raw, body) = split_frontmatter(&text);
        let fm_raw =
            fm_raw.ok_or_else(|| anyhow!("{}: OKF note has no frontmatter", path.display()))?;

        // An okf-ext-valid note deserializes directly; otherwise map loosely.
        let note = match serde_yml::from_str::<Note>(&fm_raw) {
            Ok(note) => note,
            Err(_) => {
                let fm: Value = parse_fm_tolerant(&fm_raw);
                Note {
                    note_type: get_enum::<CkfType>(&fm, "type").unwrap_or(CkfType::Doc),
                    id: get_str(&fm, "id").unwrap_or_else(|| file_stem(path)),
                    title: get_str(&fm, "title").unwrap_or_else(|| file_stem(path)),
                    updated: updated_or_mtime(Some(&fm), path),
                    description: get_str(&fm, "description"),
                    resource: get_str(&fm, "resource"),
                    tags: get_vec_str(&fm, "tags"),
                    timestamp: get_str(&fm, "timestamp"),
                    ..Note::stub()
                }
            }
        };
        Ok(Document { note, body })
    }
}
