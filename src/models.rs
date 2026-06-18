//! The `okf-ext/0.1` note model.
//!
//! A note is structurally an OKF document (one concept, `type` always present)
//! with a *richer, closed* vocabulary and *typed* links. The closed enums below
//! are sourced from the dev.tel schema (`_schema/note-types.md`) plus the
//! unified additions needed to cover the memory / skills / per-project surfaces.
//!
//! **Vocabulary discipline.** [`Note`] is `#[serde(deny_unknown_fields)]`: an
//! unknown or misspelled frontmatter key is a hard deserialize error, not a
//! silently-dropped field. For the same reason there is **no catch-all overflow
//! map** — the surface-specific `x-*` extensions are modelled as explicit
//! optional fields. Discipline (typo = error) is the whole point of the richer
//! internal form; arbitrary overflow would defeat it. Extending the vocabulary
//! is a deliberate edit here, mirroring the dev.tel "adding a field is an
//! architectural change" stance.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The unified, closed note-type axis: the dev.tel set plus the additions that
/// let the memory / skills / per-project surfaces map in. `decision` (dev.tel,
/// globally-id'd) and `adr` (per-project, numbered) stay distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CkfType {
    Project,
    Root,
    Domain,
    Decision,
    Fork,
    Log,
    Reference,
    Memory,
    Skill,
    Adr,
    Scope,
    Task,
    Index,
    Doc,
}

/// The eight workspace roots plus the decisions/forks `(cross-cutting)` sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Root {
    #[serde(rename = "personal")]
    Personal,
    #[serde(rename = "^loop")]
    Loop,
    #[serde(rename = "frameworks")]
    Frameworks,
    #[serde(rename = "exposure")]
    Exposure,
    #[serde(rename = "repositories")]
    Repositories,
    #[serde(rename = "docs")]
    Docs,
    #[serde(rename = "ops")]
    Ops,
    #[serde(rename = "archive")]
    Archive,
    #[serde(rename = "(cross-cutting)")]
    CrossCutting,
}

/// Workspace tier ladder (variant names already match the wire form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Tier {
    T0,
    T1,
    T2,
}

/// Union of the per-type status vocabularies (project / decision / fork). KC-9
/// `lint` enforces which subset is valid for a given `type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Active,
    Blocked,
    Parked,
    Shipped,
    Archived,
    Superseded,
    Reversed,
    Closed,
    Demoted,
    ReTriggered,
}

/// The cross-cutting FDE-domain axis (intentionally small).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FdeDomain {
    Hardware,
    Infra,
    Tooling,
    Product,
    Ops,
    Research,
}

/// Typed-link predicates. On OKF export these degrade to plain Markdown links
/// (the predicate survives as link text) plus a sidecar record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LinkRel {
    Related,
    DependsOn,
    Supersedes,
    SupersededBy,
    /// Auto-derived from inline / wikilinks; never hand-authored.
    Mentions,
}

/// Memory-surface sub-kind, carried as `x-mem-kind` when `type: memory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MemKind {
    User,
    Feedback,
    Project,
    Reference,
    Hardware,
}

/// A typed link to another note, by stable `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypedLink {
    pub id: String,
    pub rel: LinkRel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// An `okf-ext/0.1` note. Field order here is the canonical frontmatter order
/// (required → reserved → extension → links); KC-5 serializes in declaration
/// order for deterministic round-trips.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Note {
    // --- required ---
    #[serde(rename = "type")]
    pub note_type: CkfType,
    pub id: String,
    pub title: String,
    /// ISO `YYYY-MM-DD`. Maps to OKF `timestamp` on export.
    pub updated: String,

    // --- reserved (OKF passthrough) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// The gap-filling field every surface lacks today (back-filled from git/mtime).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    // --- extension (operator-private; dropped to sidecar on OKF export) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<Root>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<Tier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fde_domain: Vec<FdeDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tech: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hardware: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stakeholder: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_if: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,

    // --- known surface-specific extensions (explicit, to keep deny working) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_mem_kind: Option<MemKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_origin_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_when_to_use: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_argument_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_allowed_tools: Option<String>,

    // --- typed links ---
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<TypedLink>,
}

impl Note {
    /// A note with placeholder required fields and all optional fields empty —
    /// for struct-update syntax (`..Note::stub()`) in the surface adapters,
    /// which set the required fields explicitly.
    #[must_use]
    pub fn stub() -> Self {
        Self {
            note_type: CkfType::Doc,
            id: String::new(),
            title: String::new(),
            updated: String::new(),
            description: None,
            resource: None,
            tags: Vec::new(),
            timestamp: None,
            created: None,
            root: None,
            tier: None,
            status: None,
            fde_domain: Vec::new(),
            container: None,
            container_path: None,
            tech: Vec::new(),
            hardware: Vec::new(),
            stakeholder: Vec::new(),
            trigger: None,
            stop_if: None,
            supersedes: None,
            superseded_by: None,
            x_mem_kind: None,
            x_origin_session_id: None,
            x_when_to_use: None,
            x_argument_hint: None,
            x_allowed_tools: None,
            links: Vec::new(),
        }
    }
}

/// The committed JSON Schema for [`Note`]. The drift test keeps
/// `spec/okf-ext-0.1.schema.json` in sync with this.
#[must_use]
pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Note);
    serde_json::to_string_pretty(&schema).expect("Note schema serializes to JSON")
}
