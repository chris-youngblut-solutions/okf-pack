# ADR 0001 — No Kùzu graph backend; graph emits JSON/HTML

**Status:** Accepted · **Date:** 2026-06-17 · **Container:** okf-pack

## Context

`graph` (KC-11) emits a link graph from a bundle. The plan considered reusing the
dev-tel-graph KuzuDB node/edge schema via the Rust `kuzu` crate so existing Cypher
would run unmodified.

## Decision

Drop the `kuzu` crate. `graph` emits `json` and `html` only. The pack's store
stays DuckDB (KC-7).

## Rationale

The `kuzu` crate (0.11.3) fails to **link** in this environment (Fedora, gcc 15.2):
the cxx FFI bridge symbols (`kuzu_rs$cxxbridge1$value_get_internal_id`,
`…$node_value_get_node_id`, and others) are undefined at link time. CI runs
`--all-features`, so carrying it as an optional feature would also break CI. A
dedicated graph engine is not required for the format/adapter; substring search
over the DuckDB store already covers retrieval.

## Consequences

- `graph --emit {json,html}` ships; `kuzu` is not an option.
- Re-addable behind an optional feature if a working `kuzu` build environment
  appears — that would also require CI to stop using `--all-features` (or to gate
  the feature explicitly).
