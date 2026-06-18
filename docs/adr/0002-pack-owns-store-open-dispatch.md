# ADR 0002 — The pack owns its store; RPC uses an open dispatch table

**Status:** Accepted · **Date:** 2026-06-17 · **Container:** okf-pack

## Context

okf-pack ships as a Spaces pack: a component a Space mounts to provide a
`tile.knowledge-context`. Two structural choices shape how it runs inside a host:
where its persistent state lives, and how its JSON-RPC methods are registered. Both
are referenced from the code (`src/store.rs`, `src/rpc.rs`) and warrant a record.

## Decision

1. **The pack brings its own store.** It embeds DuckDB (`src/store.rs`) and writes
   its index/materialization into `.okf/` inside the Space directory. It does not
   depend on a host-provided database, a shared service, or a network store.
2. **The pack is a separate process.** It is launched as `okf-pack serve --rpc
   stdio` and speaks JSON-RPC over stdio. It never links a Spaces crate.
3. **RPC dispatch is an open table, not a closed enum.** `src/rpc.rs` registers
   methods in a `HashMap` keyed by method name. The mandatory transport methods
   (`ping` / `shutdown` / `capabilities`) and the knowledge methods are entries in
   that table; adding or swapping a method is a registration, not a match-arm.

## Rationale

- **AGPL boundary.** The Spaces substrate is AGPL + commercial dual-licensed.
  Because the pack is a separate process that never links a Spaces crate, it stays
  outside that boundary and can be Apache-2.0 OR MIT. Owning its store is part of
  the same separation — no shared library, no shared schema, no linkage.
- **Hot-swap invariant.** A host should be able to replace the knowledge-context
  pack with a different implementation (or a local-model variant) without the host
  knowing the method set at compile time. An open dispatch table makes the method
  set data, so a replacement that honors `capabilities` is drop-in. A closed enum
  would bake the method set into the host's type system and defeat the swap.
- **Portability.** An embedded store has no external dependency to provision; the
  pack works the same in CI, on a dev box, or mounted in a live Space.

## Consequences

- The pack is responsible for its own migrations and on-disk format under `.okf/`.
- The host and the pack share no types; the contract is the JSON-RPC method set +
  `package.toml` capabilities, not a Rust API.
- Unknown methods return a JSON-RPC error rather than failing to compile; method
  coverage is enforced by tests, not the type system.
- The knowledge methods (`index_space`, `query`, `export_okf`, …) are currently
  registered stubs returning `-32000`; wiring them to the store + embedder + export
  is a follow-up, and — by this ADR — is a change to table entries, not dispatch.
