//! KC-12 — `okf-pack init`: stamp a knowledge-context pack into a target dir.
//!
//! The `templates/space-pack/` cookiecutter is embedded in the binary, so `init`
//! works anywhere with no template tree on disk. It writes the pack's
//! `package.toml`, `.okf/config.toml`, a `.space/manifest.patch.toml` (to merge
//! into a Space by hand — never applied automatically), `README.md`, and
//! `.gitignore`. The operator-facing `/okf-init` skill wraps this subcommand.

use anyhow::Result;
use std::path::PathBuf;

pub struct InitOptions {
    pub target: PathBuf,
    pub name: String,
    pub year: String,
    pub author: String,
    pub description: String,
    pub license_slug: String,
    pub pack_id: String,
    pub component: String,
    pub embedder: String,
    pub embed_model: String,
    pub net_mode: String,
    pub okf_version: String,
}

impl InitOptions {
    /// Build options with sensible defaults derived from the target + inputs.
    #[must_use]
    pub fn new(target: PathBuf, pack_id: String, component: String, embedder: String) -> Self {
        let name = target
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("knowledge-pack")
            .to_string();
        let net_mode = if embedder == "local" {
            "none"
        } else {
            "frontier"
        }
        .to_string();
        Self {
            description: format!("Knowledge-context pack for {name}."),
            year: current_year(),
            author: git_author(),
            license_slug: "apache-mit-dual".into(),
            embed_model: String::new(),
            okf_version: "0.1".into(),
            name,
            target,
            pack_id,
            component,
            embedder,
            net_mode,
        }
    }
}

const FILES: &[(&str, &str)] = &[
    (
        "package.toml",
        include_str!("../templates/space-pack/package.toml.tmpl"),
    ),
    (
        ".okf/config.toml",
        include_str!("../templates/space-pack/okf-config.toml.tmpl"),
    ),
    (
        ".space/manifest.patch.toml",
        include_str!("../templates/space-pack/manifest.patch.toml.tmpl"),
    ),
    (
        "README.md",
        include_str!("../templates/space-pack/README.md.tmpl"),
    ),
    (
        ".gitignore",
        include_str!("../templates/space-pack/gitignore.tmpl"),
    ),
];

/// Stamp the pack into `opts.target`. Returns the number of files written.
pub fn init(opts: &InitOptions) -> Result<usize> {
    for (rel, template) in FILES {
        let path = opts.target.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, substitute(template, opts))?;
    }
    Ok(FILES.len())
}

fn substitute(template: &str, o: &InitOptions) -> String {
    template
        .replace("{{NAME}}", &o.name)
        .replace("{{YEAR}}", &o.year)
        .replace("{{AUTHOR}}", &o.author)
        .replace("{{DESCRIPTION}}", &o.description)
        .replace("{{LICENSE_SLUG}}", &o.license_slug)
        .replace("{{PACK_ID}}", &o.pack_id)
        .replace("{{COMPONENT}}", &o.component)
        .replace("{{EMBEDDER}}", &o.embedder)
        .replace("{{EMBED_MODEL}}", &o.embed_model)
        .replace("{{NET_MODE}}", &o.net_mode)
        .replace("{{OKF_VERSION}}", &o.okf_version)
}

fn current_year() -> String {
    time::OffsetDateTime::now_utc().year().to_string()
}

fn git_author() -> String {
    std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown".to_string())
}
