//! KC-3 — pure markdown/link helpers, ported from
//! `dev-tel-graph/ingester/parse.py` (the Garage-era `tier_from_path` /
//! `NOTE_TYPES` classification is intentionally **not** ported).
//!
//! Everything here is a pure function. [`resolve_target`] does **no** filesystem
//! access — it resolves lexically so it is deterministic and testable without a
//! tree on disk. Standard Markdown links go through `pulldown-cmark` (which
//! correctly ignores code spans/blocks and escapes); the workspace-specific
//! wikilink / tag / inline-code-path conventions use regexes, matching the
//! reference parser's behavior.

use pulldown_cmark::{Event, Parser, Tag};
use regex::Regex;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

fn fenced_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)```.*?```").unwrap())
}
fn inline_strip_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`[^`\n]+`").unwrap())
}
fn inline_capture_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`([^`\n]+)`").unwrap())
}
fn wiki_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([^\]\n]+)\]\]").unwrap())
}
fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?:^|\s)#([a-zA-Z][a-zA-Z0-9_/-]+)").unwrap())
}
fn scheme_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^[a-z]+://").unwrap())
}

/// Remove fenced code blocks (```` ``` ... ``` ````).
#[must_use]
pub fn strip_fenced(content: &str) -> String {
    fenced_re().replace_all(content, "").into_owned()
}

/// Remove inline code spans (`` `...` ``).
#[must_use]
pub fn strip_inline(content: &str) -> String {
    inline_strip_re().replace_all(content, "").into_owned()
}

/// Inline-code spans that look like `.md` file paths — the workspace convention
/// where universe maps / CLAUDE.md docs cross-reference via inline-code paths in
/// bullet lists rather than Markdown links. Glob patterns are skipped (they are
/// illustrative, not real references). Input should already be fenced-stripped.
#[must_use]
pub fn inline_code_paths(content_no_fenced: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for caps in inline_capture_re().captures_iter(content_no_fenced) {
        let inner = caps[1].trim();
        if !inner.ends_with(".md") {
            continue;
        }
        if !inner.contains('/') && !inner.starts_with('~') {
            continue;
        }
        if inner.contains('*') || inner.contains('?') {
            continue;
        }
        paths.push(inner.to_string());
    }
    paths
}

/// Standard Markdown link destinations (`[text](dest)`), via `pulldown-cmark`.
/// Robustly skips code spans/blocks and handles escapes, so it operates on the
/// raw content (no pre-stripping needed).
#[must_use]
pub fn markdown_link_dests(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for event in Parser::new(content) {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            out.push(dest_url.into_string());
        }
    }
    out
}

/// Wikilink targets (`[[name]]`). Input should be prose (code-stripped).
#[must_use]
pub fn wikilinks(prose: &str) -> Vec<String> {
    wiki_re()
        .captures_iter(prose)
        .map(|c| c[1].trim().to_string())
        .collect()
}

/// Hash tags (`#tag`). Input should be prose (code-stripped).
#[must_use]
pub fn tags(prose: &str) -> Vec<String> {
    tag_re()
        .captures_iter(prose)
        .map(|c| c[1].to_string())
        .collect()
}

/// The raw link surface of a note body, with the reference parser's two-pass
/// code handling: Markdown links + inline-code paths come from the
/// fenced-stripped text; wikilinks + tags come from fully code-stripped prose.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BodyLinks {
    pub markdown_dests: Vec<String>,
    pub inline_code_paths: Vec<String>,
    pub wikilinks: Vec<String>,
    pub tags: Vec<String>,
}

/// Extract the full raw link surface of a Markdown body (no resolution — that is
/// per-file and lives in the surface adapters, KC-4).
#[must_use]
pub fn body_links(content: &str) -> BodyLinks {
    let no_fenced = strip_fenced(content);
    let prose = strip_inline(&no_fenced);
    BodyLinks {
        markdown_dests: markdown_link_dests(content),
        inline_code_paths: inline_code_paths(&no_fenced),
        wikilinks: wikilinks(&prose),
        tags: tags(&prose),
    }
}

/// Resolve a link target to a path relative to `tree_root`, lexically.
///
/// Returns `None` for empty / external (`scheme://`) / `mailto:` / anchor-only
/// targets, and for anything that escapes `tree_root`. `~/` expands to `$HOME`;
/// a leading `/` is absolute; otherwise the target is resolved relative to
/// `src_abs`'s parent. No filesystem access — `..` is collapsed lexically.
#[must_use]
pub fn resolve_target(src_abs: &Path, tree_root: &Path, target: &str) -> Option<String> {
    if target.is_empty() {
        return None;
    }
    let clean = target.split('#').next().unwrap_or("");
    let clean = clean.split('?').next().unwrap_or("");
    if clean.is_empty() || scheme_re().is_match(clean) || clean.starts_with("mailto:") {
        return None;
    }

    let target_abs: PathBuf = if let Some(rest) = clean.strip_prefix("~/") {
        home_dir()?.join(rest)
    } else if clean.starts_with('/') {
        PathBuf::from(clean)
    } else {
        src_abs.parent().unwrap_or(src_abs).join(clean)
    };

    let normalized = normalize_lexical(&target_abs);
    let rel = normalized.strip_prefix(tree_root).ok()?;
    Some(rel.to_string_lossy().into_owned())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Collapse `.` and `..` components without touching the filesystem
/// (`os.path.normpath` semantics: cannot ascend above the root).
fn normalize_lexical(p: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => out.push(comp),
            },
            other => out.push(other),
        }
    }
    out.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_strip_fenced_and_inline() {
        let md = "a\n```\nfenced `not stripped` here\n```\nb `inline` c";
        let no_fenced = strip_fenced(md);
        assert!(!no_fenced.contains("fenced"));
        assert!(no_fenced.contains("`inline`"));
        assert_eq!(strip_inline(&no_fenced).trim(), "a\n\nb  c");
    }

    #[test]
    fn parse_markdown_links_skip_code() {
        let md = "see [spec](okf.md) and `[notalink](x.md)` and [ext](https://e.com)";
        let dests = markdown_link_dests(md);
        assert!(dests.contains(&"okf.md".to_string()));
        assert!(dests.contains(&"https://e.com".to_string()));
        assert!(
            !dests.iter().any(|d| d == "x.md"),
            "code-span link must be ignored"
        );
    }

    #[test]
    fn parse_wikilinks_and_tags() {
        let prose = "ties [[panel-tiling]] and [[okf-pack]] #tooling #ag/sprayer";
        assert_eq!(wikilinks(prose), vec!["panel-tiling", "okf-pack"]);
        assert_eq!(tags(prose), vec!["tooling", "ag/sprayer"]);
    }

    #[test]
    fn parse_inline_code_paths_heuristic() {
        let s = "`frameworks/x.md` `~/C1-10P/y.md` `plain.md` `*/glob-*.md` `nota path`";
        let got = inline_code_paths(s);
        assert_eq!(got, vec!["frameworks/x.md", "~/C1-10P/y.md"]);
    }

    #[test]
    fn parse_resolve_target_relative_and_escape() {
        let root = Path::new("/tmp/treeroot");
        let src = Path::new("/tmp/treeroot/a/b.md");
        assert_eq!(
            resolve_target(src, root, "../c.md").as_deref(),
            Some("c.md")
        );
        assert_eq!(
            resolve_target(src, root, "./d.md").as_deref(),
            Some("a/d.md")
        );
        // escapes the tree → None
        assert_eq!(resolve_target(src, root, "../../../etc/passwd"), None);
    }

    #[test]
    fn parse_resolve_target_rejects_external_and_anchor() {
        let root = Path::new("/tmp/treeroot");
        let src = Path::new("/tmp/treeroot/a/b.md");
        assert_eq!(resolve_target(src, root, "https://example.com"), None);
        assert_eq!(resolve_target(src, root, "mailto:x@y.z"), None);
        assert_eq!(resolve_target(src, root, "#anchor-only"), None);
        assert_eq!(resolve_target(src, root, ""), None);
    }

    #[test]
    fn parse_resolve_target_home_expansion() {
        let home = home_dir().expect("HOME set in test env");
        let root = home.join("C1-10P");
        let src = root.join("x.md");
        assert_eq!(
            resolve_target(&src, &root, "~/C1-10P/sub/y.md").as_deref(),
            Some("sub/y.md")
        );
    }
}
