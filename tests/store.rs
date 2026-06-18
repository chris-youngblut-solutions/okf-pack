//! KC-7 — DuckDB store: index + incremental + query.

use okf_pack::store::Store;
use std::path::Path;

fn seed_space() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("okfpack-test-store");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-space");
    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
    }
    dir
}

#[test]
fn store_index_incremental_and_query() {
    let dir = seed_space();
    let store = Store::open(&dir).unwrap();

    let first = store.index_space(&dir).unwrap();
    assert_eq!(first.indexed, 3, "three sample-space notes");
    assert_eq!(store.count().unwrap(), 3);

    // Re-index with no changes → all skipped (incremental by content hash).
    let second = store.index_space(&dir).unwrap();
    assert_eq!(second.indexed, 0);
    assert_eq!(second.skipped, 3);

    // Substring query hits the title "Open Knowledge Format".
    let hits = store.query("Knowledge", 10).unwrap();
    assert!(
        hits.iter().any(|h| h.id == "okf-spec"),
        "expected okf-spec, got {hits:?}"
    );
}
