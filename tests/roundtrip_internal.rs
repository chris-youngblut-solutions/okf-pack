//! KC-5 — internal⇄internal round-trip is lossless.
//!
//! A property test (random valid notes) plus a full-field example that exercises
//! YAML quoting edge cases (colons, spaces, unicode).

use okf_pack::canonical::{document_to_string, parse_document};
use okf_pack::models::{CkfType, FdeDomain, LinkRel, MemKind, Note, Root, Status, Tier, TypedLink};
use okf_pack::surface::Document;
use proptest::prelude::*;

fn ckftype() -> impl Strategy<Value = CkfType> {
    prop_oneof![
        Just(CkfType::Project),
        Just(CkfType::Decision),
        Just(CkfType::Memory),
        Just(CkfType::Skill),
        Just(CkfType::Adr),
        Just(CkfType::Index),
        Just(CkfType::Doc),
    ]
}

prop_compose! {
    fn link()(
        id in "[a-z][a-z0-9-]{0,15}",
        which in 0u8..5,
        note in prop::option::of("[a-z][a-z0-9 -]{0,15}"),
    ) -> TypedLink {
        let rel = match which {
            0 => LinkRel::Related,
            1 => LinkRel::DependsOn,
            2 => LinkRel::Supersedes,
            3 => LinkRel::SupersededBy,
            _ => LinkRel::Mentions,
        };
        TypedLink { id, rel, note }
    }
}

prop_compose! {
    fn note_strategy()(
        note_type in ckftype(),
        id in "[a-z][a-z0-9-]{0,20}",
        title in "[A-Za-z][A-Za-z0-9-]{0,20}",
        updated in "[0-9]{4}-[0-9]{2}-[0-9]{2}",
        description in prop::option::of("[a-z][a-z0-9 -]{0,30}"),
        tags in prop::collection::vec("[a-z][a-z0-9-]{0,10}", 0..3),
        created in prop::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}"),
        tech in prop::collection::vec("[a-z][a-z0-9-]{0,10}", 0..3),
        stakeholder in prop::collection::vec("[a-z][a-z0-9-]{0,10}", 0..3),
        links in prop::collection::vec(link(), 0..4),
    ) -> Note {
        Note {
            note_type, id, title, updated,
            description, tags, created, tech, stakeholder, links,
            ..Note::stub()
        }
    }
}

proptest! {
    #[test]
    fn roundtrip_internal_property(note in note_strategy()) {
        let doc = Document { note: note.clone(), body: "Body line one.\nBody line two.".to_string() };
        let serialized = document_to_string(&doc).expect("serialize");
        let parsed = parse_document(&serialized).expect("parse");
        prop_assert_eq!(&parsed.note, &note);
        prop_assert_eq!(&parsed.body, &doc.body);
    }
}

#[test]
fn roundtrip_internal_full_field_example() {
    // Every field populated, with YAML-tricky strings.
    let note = Note {
        note_type: CkfType::Decision,
        id: "0001-pick-format".into(),
        title: "Use okf-ext: a colon, spaces & a 🎯".into(),
        updated: "2026-06-17".into(),
        description: Some("Has: a colon, comma, and trailing space ".into()),
        resource: Some("https://example.invalid/x?y=1#z".into()),
        tags: vec!["okf".into(), "format".into()],
        timestamp: Some("2026-06-17T14:30:00Z".into()),
        created: Some("2026-06-01".into()),
        root: Some(Root::Loop),
        tier: Some(Tier::T1),
        status: Some(Status::Active),
        fde_domain: vec![FdeDomain::Tooling, FdeDomain::Product],
        container: Some("okf-pack".into()),
        container_path: Some("^loop/dev-scoping/okf-pack".into()),
        tech: vec!["Rust".into()],
        hardware: vec![],
        stakeholder: vec!["self".into()],
        trigger: Some("a client needs OKF export".into()),
        stop_if: Some("no external consumer".into()),
        supersedes: None,
        superseded_by: None,
        x_mem_kind: Some(MemKind::Feedback),
        x_origin_session_id: Some("sess-123".into()),
        x_when_to_use: Some("when exporting".into()),
        x_argument_hint: Some("<bundle>".into()),
        x_allowed_tools: Some("Bash, Read".into()),
        links: vec![
            TypedLink {
                id: "concept-b".into(),
                rel: LinkRel::DependsOn,
                note: Some("needs B".into()),
            },
            TypedLink {
                id: "concept-c".into(),
                rel: LinkRel::Mentions,
                note: None,
            },
        ],
    };
    let doc = Document {
        note: note.clone(),
        body: "## Context\n\nProse with `code`.".to_string(),
    };
    let serialized = document_to_string(&doc).expect("serialize");
    let parsed = parse_document(&serialized).expect("parse");
    assert_eq!(parsed.note, note);
    assert_eq!(parsed.body, doc.body);
}
