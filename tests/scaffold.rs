//! KC-12 — `okf-pack init` stamps a valid knowledge-context pack.

use okf_pack::scaffold::{InitOptions, init};
use std::path::PathBuf;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn init_scaffold_stamps_valid_pack() {
    let target = temp("okfpack-test-init");
    let opts = InitOptions::new(
        target.clone(),
        "client.kc".into(),
        "kc".into(),
        "local".into(),
    );

    let count = init(&opts).unwrap();
    assert_eq!(count, 5, "five files stamped");

    // package.toml: substituted and a valid Spaces manifest.
    let pkg = std::fs::read_to_string(target.join("package.toml")).unwrap();
    assert!(!pkg.contains("{{"), "no unsubstituted placeholders: {pkg}");
    let manifest: toml::Value = toml::from_str(&pkg).unwrap();
    assert_eq!(
        manifest.get("id").and_then(|v| v.as_str()),
        Some("client.kc")
    );
    assert_eq!(
        manifest.get("provides").and_then(|v| v.as_str()),
        Some("tile.kc")
    );
    assert_eq!(
        manifest
            .get("capabilities")
            .and_then(|c| c.get("net"))
            .and_then(|v| v.as_str()),
        Some("none"),
        "local embedder => net none"
    );

    // The manifest patch and config parse as TOML.
    let patch = std::fs::read_to_string(target.join(".space/manifest.patch.toml")).unwrap();
    let _: toml::Value = toml::from_str(&patch).unwrap();
    let config = std::fs::read_to_string(target.join(".okf/config.toml")).unwrap();
    let _: toml::Value = toml::from_str(&config).unwrap();

    assert!(target.join("README.md").exists());
    assert!(target.join(".gitignore").exists());
}
