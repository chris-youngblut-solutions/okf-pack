//! KC-8 — the privilege gate.
//!
//! A denylist of regex patterns sourced from `~/.claude/ship-prep/denylist.md`
//! (the workspace's single source of truth). The gate is **dual-use**: it blocks
//! both OKF export to an external target (`--target okf-gcp`) and, later, any
//! external embedder (KC-10). Local `--target files-only` is always allowed, so
//! privileged corpora never leave the box by either path.

use anyhow::Result;
use regex::Regex;
use std::path::PathBuf;

pub struct Denylist {
    patterns: Vec<Regex>,
}

impl Denylist {
    /// Build from explicit patterns (used by tests — hermetic, no real file).
    pub fn from_patterns(patterns: &[&str]) -> Result<Self> {
        let patterns = patterns
            .iter()
            .map(|p| Regex::new(p))
            .collect::<Result<_, _>>()?;
        Ok(Self { patterns })
    }

    /// Load the patterns from the workspace denylist (the fenced `denylist-*`
    /// regex blocks). A missing file yields an empty (permissive) denylist.
    pub fn load() -> Result<Self> {
        let text = std::fs::read_to_string(denylist_path()).unwrap_or_default();
        let patterns = extract_denylist_patterns(&text)
            .into_iter()
            .filter_map(|p| Regex::new(&p).ok())
            .collect();
        Ok(Self { patterns })
    }

    /// The patterns that match anywhere in `text`.
    #[must_use]
    pub fn scan(&self, text: &str) -> Vec<String> {
        self.patterns
            .iter()
            .filter(|re| re.is_match(text))
            .map(|re| re.as_str().to_string())
            .collect()
    }

    #[must_use]
    pub fn is_clean(&self, text: &str) -> bool {
        !self.patterns.iter().any(|re| re.is_match(text))
    }
}

fn denylist_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".claude/ship-prep/denylist.md")
}

/// Extract pattern lines from ```` ```denylist-* ```` fenced blocks.
fn extract_denylist_patterns(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            let lang = trimmed.trim_start_matches('`');
            if in_block {
                in_block = false;
            } else if lang.starts_with("denylist") {
                in_block = true;
            }
            continue;
        }
        if in_block && !trimmed.is_empty() {
            out.push(trimmed.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures use synthetic placeholder terms only — never a real denylist
    // entry — so the source itself stays clean for public release.
    #[test]
    fn privilege_extracts_fenced_patterns() {
        let md = "intro\n\n```denylist-hard\n\\bWidgetCo\\b\n\\bglorp\\b\n```\n\n```rust\nignored\n```\n";
        let pats = extract_denylist_patterns(md);
        assert_eq!(pats, vec!["\\bWidgetCo\\b", "\\bglorp\\b"]);
    }

    #[test]
    fn privilege_scan_matches() {
        let dl = Denylist::from_patterns(&[r"\bWidgetCo\b"]).unwrap();
        assert!(!dl.is_clean("uses WidgetCo here"));
        assert!(dl.is_clean("nothing sensitive"));
    }
}
