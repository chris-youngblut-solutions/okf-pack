//! KC-9 — rule-tagged bundle validation.

use okf_pack::validate::validate_bundle;
use std::path::{Path, PathBuf};

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

#[test]
fn validate_clean_bundle_has_no_findings() {
    let findings = validate_bundle(&fixture("sample-space")).unwrap();
    assert!(
        findings.is_empty(),
        "sample-space should be clean: {findings:?}"
    );
}

#[test]
fn validate_corrupt_fixture_flags_schema_rule() {
    let findings = validate_bundle(&fixture("corrupt")).unwrap();
    assert!(
        findings.iter().any(|f| f.rule == "E002-schema"),
        "corrupt fixture should trip E002-schema: {findings:?}"
    );
}
