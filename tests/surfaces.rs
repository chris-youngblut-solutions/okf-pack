//! KC-4 — surface adapter tests, against synthetic fixtures only.

use okf_pack::models::{CkfType, LinkRel, MemKind, Status};
use okf_pack::surface::{adapter_for, migrate, validate_bundle};
use std::path::{Path, PathBuf};

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

#[test]
fn surfaces_okf_migrate_then_validate() {
    let src = fixture("sample-space");
    let dest = std::env::temp_dir().join("okfpack-test-surfaces-okf");
    let _ = std::fs::remove_dir_all(&dest);

    let count = migrate("okf", &src, &dest).expect("migrate okf surface");
    assert_eq!(count, 3, "three sample-space notes");

    let errors = validate_bundle(&dest).expect("validate the emitted bundle");
    assert!(
        errors.is_empty(),
        "emitted bundle must be valid okf-ext: {errors:?}"
    );
}

#[test]
fn surfaces_memory_bimodal_reader() {
    let adapter = adapter_for("memory").unwrap();
    let root = fixture("surfaces/memory");

    let flat = adapter
        .read_note(&root.join("flat-fact.md"), &root)
        .unwrap();
    assert_eq!(flat.note.note_type, CkfType::Memory);
    assert_eq!(flat.note.x_mem_kind, Some(MemKind::Feedback));
    assert!(
        flat.note
            .links
            .iter()
            .any(|l| l.rel == LinkRel::Mentions && l.id == "sample-nested-fact"),
        "body wikilink becomes a mention"
    );

    let nested = adapter
        .read_note(&root.join("nested-fact.md"), &root)
        .unwrap();
    assert_eq!(
        nested.note.x_mem_kind,
        Some(MemKind::Project),
        "metadata.type read"
    );

    let index = adapter.read_note(&root.join("MEMORY.md"), &root).unwrap();
    assert_eq!(
        index.note.note_type,
        CkfType::Index,
        "MEMORY.md becomes the index"
    );
}

#[test]
fn surfaces_skills_infers_skill_type() {
    let adapter = adapter_for("skills").unwrap();
    let root = fixture("surfaces/skills");

    let files = adapter.discover(&root).unwrap();
    assert_eq!(files.len(), 1, "one SKILL.md");

    let doc = adapter.read_note(&files[0], &root).unwrap();
    assert_eq!(doc.note.note_type, CkfType::Skill);
    assert_eq!(
        doc.note.x_when_to_use.as_deref(),
        Some("when the user says \"example\"")
    );
    assert_eq!(doc.note.x_allowed_tools.as_deref(), Some("Bash, Read"));
}

#[test]
fn surfaces_devtel_passthrough_and_links() {
    let adapter = adapter_for("devtel").unwrap();
    assert!(adapter.frozen_casing(), "dev.tel casing is frozen");

    let root = fixture("surfaces/devtel");
    let doc = adapter
        .read_note(&root.join("project-note.md"), &root)
        .unwrap();
    assert_eq!(doc.note.note_type, CkfType::Project);
    assert_eq!(doc.note.fde_domain.len(), 2);
    assert!(
        doc.note
            .links
            .iter()
            .any(|l| l.rel == LinkRel::Related && l.id == "other-project")
    );
    assert!(
        doc.note
            .links
            .iter()
            .any(|l| l.rel == LinkRel::DependsOn && l.id == "some-dependency")
    );
    assert!(doc.note.created.is_some(), "created back-filled");
}

#[test]
fn surfaces_tolerate_unquoted_colons_in_frontmatter() {
    // Regression: real Claude-Code memory/skill frontmatter carries unquoted
    // colons in scalar values (`description: …it's live: resume…`), which strict
    // YAML rejects. The adapters must read them anyway (strict-first, then a
    // lenient line parser) instead of aborting the whole migration.
    let dir = std::env::temp_dir().join("okfpack-test-colon-fm");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // memory: unquoted colon in `description` + a nested `metadata:` block.
    let mem = dir.join("colon-mem.md");
    std::fs::write(
        &mem,
        "---\nname: colon-mem\ndescription: don't archive — it's live: jobs under exposure/\nmetadata:\n  node_type: memory\n  type: reference\n  originSessionId: sess-9\n---\nBody with a [[other-fact]] link.\n",
    )
    .unwrap();
    let m = adapter_for("memory")
        .unwrap()
        .read_note(&mem, &dir)
        .unwrap();
    assert_eq!(m.note.note_type, CkfType::Memory);
    assert_eq!(
        m.note.x_mem_kind,
        Some(MemKind::Reference),
        "nested metadata.type read despite the unquoted colon in description"
    );
    assert_eq!(
        m.note.description.as_deref(),
        Some("don't archive — it's live: jobs under exposure/"),
        "value kept verbatim past the inner colon"
    );
    assert_eq!(m.note.x_origin_session_id.as_deref(), Some("sess-9"));

    // skills: unquoted colon in `when_to_use`.
    let skdir = dir.join("colon-skill");
    std::fs::create_dir_all(&skdir).unwrap();
    let sk = skdir.join("SKILL.md");
    std::fs::write(
        &sk,
        "---\nname: colon-skill\ndescription: does a thing\nwhen_to_use: User says \"/x\". Also: when handing off offline\nallowed-tools: Bash, Read\n---\nSkill body.\n",
    )
    .unwrap();
    let s = adapter_for("skills")
        .unwrap()
        .read_note(&sk, &skdir)
        .unwrap();
    assert_eq!(s.note.note_type, CkfType::Skill);
    assert_eq!(
        s.note.x_when_to_use.as_deref(),
        Some("User says \"/x\". Also: when handing off offline"),
        "when_to_use kept verbatim past the inner colon"
    );
    assert_eq!(s.note.x_allowed_tools.as_deref(), Some("Bash, Read"));
}

#[test]
fn surfaces_container_synthesizes_frontmatter() {
    let adapter = adapter_for("container").unwrap();
    let root = fixture("surfaces/container");
    let doc = adapter
        .read_note(&root.join("adr/0001-sample-decision.md"), &root)
        .unwrap();

    assert_eq!(
        doc.note.note_type,
        CkfType::Adr,
        "type inferred from the adr/ dir"
    );
    assert_eq!(doc.note.id, "sample-project-0001");
    assert_eq!(
        doc.note.status,
        Some(Status::Active),
        "Accepted maps to active"
    );
    assert_eq!(doc.note.created.as_deref(), Some("2026-06-10"));
    assert!(
        doc.body.contains("# ADR 1 — Adopt okf-pack"),
        "body kept verbatim"
    );
}
