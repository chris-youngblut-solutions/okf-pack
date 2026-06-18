//! KC-9 — internal → OKF → internal round-trip (lossless with sidecar).

use okf_pack::canonical::parse_document;
use okf_pack::okf::{Target, export, import};
use okf_pack::privilege::Denylist;
use okf_pack::surface::Document;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn load_docs(dir: &Path) -> BTreeMap<String, Document> {
    let mut docs = BTreeMap::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let doc = parse_document(&std::fs::read_to_string(&path).unwrap()).unwrap();
        docs.insert(doc.note.id.clone(), doc);
    }
    docs
}

#[test]
fn roundtrip_okf_sidecar_is_lossless() {
    let src = fixture("sample-space");
    let okf_dir = temp("okfpack-rt-okf");
    let back = temp("okfpack-rt-internal");
    let denylist = Denylist::from_patterns(&[]).unwrap();

    export(&src, &okf_dir, Target::FilesOnly, &denylist, true).unwrap();
    import(&okf_dir, &back).unwrap();

    let original = load_docs(&src);
    let restored = load_docs(&back);
    assert_eq!(original.len(), restored.len(), "same note count");
    for (id, orig) in &original {
        let got = restored
            .get(id)
            .unwrap_or_else(|| panic!("missing note {id}"));
        assert_eq!(got.note, orig.note, "note `{id}` model not lossless");
        assert_eq!(
            got.body.trim(),
            orig.body.trim(),
            "note `{id}` body not lossless"
        );
    }
}

#[test]
fn roundtrip_okf_no_sidecar_is_lossy_but_valid() {
    let src = fixture("sample-space");
    let okf_dir = temp("okfpack-rt-nosc");
    let back = temp("okfpack-rt-nosc-in");
    let denylist = Denylist::from_patterns(&[]).unwrap();

    export(&src, &okf_dir, Target::FilesOnly, &denylist, false).unwrap();
    let count = import(&okf_dir, &back).unwrap();
    assert_eq!(count, 3);

    // Loose import drops extensions/links but is still schema-valid internal.
    let errors = okf_pack::surface::validate_bundle(&back).unwrap();
    assert!(errors.is_empty(), "loose import still valid: {errors:?}");
}
