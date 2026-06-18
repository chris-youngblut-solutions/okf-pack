//! KC-2 — model + schema tests.
//!
//! Covers: (1) the committed JSON Schema stays in sync with the serde structs
//! (drift gate), (2) `deny_unknown_fields` makes a typo a hard error (the
//! vocab-discipline guarantee), and (3) a minimal note round-trips.

use okf_pack::models::{CkfType, Note};
use std::path::{Path, PathBuf};

fn schema_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/okf-ext-0.1.schema.json")
}

/// The committed schema must equal what the structs generate. Run
/// `REGEN_SCHEMA=1 cargo test --test models` to regenerate after a model change.
#[test]
fn models_schema_in_sync() {
    let generated = okf_pack::models::schema_json();
    let path = schema_path();

    if std::env::var_os("REGEN_SCHEMA").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &generated).unwrap();
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing {} — run `REGEN_SCHEMA=1 cargo test --test models`",
            path.display()
        )
    });
    assert_eq!(
        generated.trim(),
        committed.trim(),
        "schema drift: run `REGEN_SCHEMA=1 cargo test --test models` and commit"
    );
}

/// A misspelled frontmatter key (`fde_domain` instead of `fde-domain`) must be a
/// hard error — the discipline the richer internal form exists to provide.
#[test]
fn models_typo_is_rejected() {
    let yaml = "\
type: project
id: panel-tiling
title: Panel Tiling
updated: 2026-05-23
fde_domain: [tooling]
";
    let parsed: Result<Note, _> = serde_yml::from_str(yaml);
    assert!(
        parsed.is_err(),
        "deny_unknown_fields must reject the `fde_domain` typo"
    );
}

/// The correctly-spelled kebab-case key parses into the closed enum.
#[test]
fn models_kebab_axis_parses() {
    let yaml = "\
type: project
id: panel-tiling
title: Panel Tiling
updated: 2026-05-23
fde-domain: [tooling, product]
root: ^loop
tier: T1
status: active
";
    let note: Note = serde_yml::from_str(yaml).expect("valid note parses");
    assert_eq!(note.note_type, CkfType::Project);
    assert_eq!(note.fde_domain.len(), 2);
}

/// A minimal note (only the four required fields) round-trips through serde.
#[test]
fn models_minimal_note_roundtrips() {
    let yaml = "\
type: reference
id: okf-spec
title: Open Knowledge Format
updated: 2026-06-17
";
    let note: Note = serde_yml::from_str(yaml).expect("minimal note parses");
    let reser = serde_yml::to_string(&note).expect("note re-serializes");
    let again: Note = serde_yml::from_str(&reser).expect("re-serialized note parses");
    assert_eq!(note, again);
}
