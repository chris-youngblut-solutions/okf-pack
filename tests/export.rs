//! KC-8 — OKF export + sidecar, and the dual privilege gate.

use okf_pack::okf::{Target, export};
use okf_pack::privilege::Denylist;
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

#[test]
fn export_emits_okf_bundle_with_sidecar() {
    let src = fixture("sample-space");
    let dest = temp("okfpack-test-export");
    let denylist = Denylist::from_patterns(&[]).unwrap();

    let report = export(&src, &dest, Target::FilesOnly, &denylist, true).unwrap();
    assert_eq!(report.item_count, 3);
    assert_eq!(report.sha256.len(), 64);

    assert!(dest.join("ckf.toml").exists(), "bundle manifest");
    assert!(dest.join(".ckf/links.json").exists(), "links sidecar");
    assert!(dest.join(".ckf/typemap.json").exists(), "typemap sidecar");

    // The OKF projection carries `timestamp`, not `updated`, and drops extensions.
    let okf = std::fs::read_to_string(dest.join("okf-spec.md")).unwrap();
    assert!(okf.contains("timestamp:"), "okf note has timestamp");
    assert!(!okf.contains("\nupdated:"), "okf note drops `updated`");
    // The typed link degrades to a `## Related` Markdown link.
    assert!(
        okf.contains("## Related"),
        "typed link surfaced as markdown"
    );
    assert!(okf.contains("[concept-b](concept-b.md)"));
}

#[test]
fn privilege_gate_blocks_okf_gcp_but_allows_files_only() {
    let src = fixture("privileged");
    let denylist = Denylist::from_patterns(&["MARKER-PRIVILEGED-XYZ"]).unwrap();

    // External target is refused for privileged content.
    let blocked = export(
        &src,
        &temp("okfpack-test-priv-gcp"),
        Target::OkfGcp,
        &denylist,
        true,
    );
    assert!(blocked.is_err(), "okf-gcp must refuse privileged content");

    // Local files-only is always allowed, even for privileged content.
    let allowed = export(
        &src,
        &temp("okfpack-test-priv-files"),
        Target::FilesOnly,
        &denylist,
        true,
    );
    assert!(allowed.is_ok(), "files-only must be allowed: {allowed:?}");
}
