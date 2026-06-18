//! KC-7 — the pack's own store (embedded DuckDB).
//!
//! Per ADR 0002 the pack brings its own store; this is it. `index_space` walks a
//! Space directory, parses each note's frontmatter into a row, and is
//! incremental by content hash. `query` is a simple substring search. The store
//! file lives at `<space>/.okf/store.duckdb` (the `.okf/` dir is skipped by the
//! walker, so the store never indexes itself).

use crate::models::Note;
use crate::surface::{discover_md, split_frontmatter};
use anyhow::Result;
use duckdb::{Connection, params};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexStats {
    pub indexed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub id: String,
    pub note_type: String,
    pub title: String,
}

impl Store {
    /// Open (creating if needed) the store under `<space_dir>/.okf/store.duckdb`.
    pub fn open(space_dir: &Path) -> Result<Self> {
        let okf = space_dir.join(".okf");
        std::fs::create_dir_all(&okf)?;
        let conn = Connection::open(okf.join("store.duckdb"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                type TEXT,
                title TEXT,
                description TEXT,
                path TEXT,
                updated TEXT,
                hash TEXT,
                body TEXT
            );",
        )?;
        Ok(Self { conn })
    }

    /// Walk `space_dir`, indexing every `.md` whose frontmatter parses as a note.
    /// Incremental: unchanged content (same hash) is skipped.
    pub fn index_space(&self, space_dir: &Path) -> Result<IndexStats> {
        let mut stats = IndexStats::default();
        for file in discover_md(space_dir)? {
            let text = std::fs::read_to_string(&file)?;
            let (Some(frontmatter), body) = split_frontmatter(&text) else {
                continue;
            };
            let Ok(note) = serde_yml::from_str::<Note>(&frontmatter) else {
                continue;
            };
            let hash = content_hash(&text);

            let existing: Option<String> = self
                .conn
                .query_row(
                    "SELECT hash FROM notes WHERE id = ?",
                    params![note.id],
                    |r| r.get(0),
                )
                .ok();
            if existing.as_deref() == Some(hash.as_str()) {
                stats.skipped += 1;
                continue;
            }

            let rel = file
                .strip_prefix(space_dir)
                .unwrap_or(&file)
                .to_string_lossy()
                .into_owned();
            self.conn
                .execute("DELETE FROM notes WHERE id = ?", params![note.id])?;
            self.conn.execute(
                "INSERT INTO notes (id, type, title, description, path, updated, hash, body)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    note.id,
                    type_str(&note),
                    note.title,
                    note.description,
                    rel,
                    note.updated,
                    hash,
                    body
                ],
            )?;
            stats.indexed += 1;
        }
        Ok(stats)
    }

    /// Substring search over title / description / body. Returns up to `k` hits.
    pub fn query(&self, q: &str, k: usize) -> Result<Vec<Hit>> {
        let pattern = format!("%{q}%");
        let mut stmt = self.conn.prepare(
            "SELECT id, type, title FROM notes
             WHERE title ILIKE ? OR description ILIKE ? OR body ILIKE ?
             LIMIT ?",
        )?;
        let rows = stmt.query_map(params![pattern, pattern, pattern, k as i64], |r| {
            Ok(Hit {
                id: r.get(0)?,
                note_type: r.get(1)?,
                title: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Number of indexed notes.
    pub fn count(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))?;
        Ok(usize::try_from(n).unwrap_or(0))
    }
}

fn type_str(note: &Note) -> String {
    serde_json::to_value(note.note_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn content_hash(s: &str) -> String {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
