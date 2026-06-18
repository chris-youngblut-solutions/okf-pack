//! KC-0b — validate the Spaces pack manifest (`package.toml`) against the
//! Spaces `package-toml-v1` pack-manifest contract: required keys present and
//! the `provides` capability is `tile.`-prefixed (namespace dispatch invariant).

use std::path::Path;

fn load_manifest() -> toml::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("package.toml");
    let text = std::fs::read_to_string(&path).expect("package.toml is present at repo root");
    toml::from_str::<toml::Value>(&text).expect("package.toml is valid TOML")
}

#[test]
fn manifest_required_keys() {
    let m = load_manifest();

    assert!(
        m.get("id").and_then(toml::Value::as_str).is_some(),
        "top-level `id` string"
    );
    assert!(
        m.get("provides").and_then(toml::Value::as_str).is_some(),
        "top-level `provides` string"
    );

    let runtime = m
        .get("runtime")
        .and_then(toml::Value::as_table)
        .expect("[runtime] table");
    for key in ["kind", "cmd", "io"] {
        assert!(runtime.get(key).is_some(), "runtime.{key} is required");
    }

    let binds = m
        .get("binds")
        .and_then(toml::Value::as_table)
        .expect("[binds] table");
    assert!(binds.get("cwd").is_some(), "binds.cwd is required");

    let caps = m
        .get("capabilities")
        .and_then(toml::Value::as_table)
        .expect("[capabilities] table");
    for key in ["fs", "net"] {
        assert!(caps.get(key).is_some(), "capabilities.{key} is required");
    }
}

#[test]
fn manifest_provides_is_tile_prefixed() {
    let m = load_manifest();
    let provides = m
        .get("provides")
        .and_then(toml::Value::as_str)
        .expect("`provides` string");
    assert!(
        provides
            .strip_prefix("tile.")
            .is_some_and(|bare| !bare.is_empty()),
        "`provides` must be `tile.`-prefixed with a non-empty capability: got {provides:?}"
    );
}
