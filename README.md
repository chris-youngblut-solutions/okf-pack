# okf-pack

okf-pack is a Rust CLI and library that reads knowledge surfaces into a unified internal
format (`okf-ext/0.1`), converts to and from Google's Open Knowledge Format (OKF),
validates and graphs bundles, and serves a JSON-RPC pack for a Space.

A **surface** is any directory of Markdown/YAML knowledge. Five adapters ship
built-in — `memory`, `skills`, `devtel`, `container`, `okf` — which read common
knowledge-surface schemas (a flat/nested memory store, a skills tree, a dev-journal
vault, per-project ADR/scope/task docs, an OKF bundle); they cover the operator's
Claude Code workspace. The CLI exposes those five. The `SurfaceAdapter` trait
(`src/surface.rs`) is public: a custom surface is a library extension — implement
the trait and call `migrate()` — not a runtime CLI plugin.

## What it does

- **Read surfaces.** `migrate-surface` reads a memory dir, a skills tree, a dev.tel
  vault, per-project ADR/scope/task docs, or an existing OKF bundle into an
  `okf-ext` bundle (a directory of Markdown files with YAML frontmatter).
- **Export / import OKF.** `export` downgrades to pure OKF (extension fields move to
  a `.ckf/` sidecar; typed links degrade to a `## Related` section). `import`
  reverses it losslessly when the sidecar is present.
- **Validate.** `validate` checks a bundle against the schema and reports
  rule-tagged findings (`E001`–`E004`); `roundtrip` checks lossless internal
  round-trips.
- **Graph.** `graph` emits the link graph as JSON or a self-contained HTML viz.
- **Serve.** `serve --rpc stdio` runs the Spaces pack (JSON-RPC over stdio). The
  transport, `ping`, `shutdown`, and `capabilities` are live; the knowledge methods
  (`index_space`, `query`, `export_okf`, …) are currently registered stubs (they
  return `-32000`). Wiring them to the store + embedder + export is follow-up work;
  see [`docs/adr/0002`](docs/adr/0002-pack-owns-store-open-dispatch.md) for the
  dispatch architecture.
- **Scaffold.** `init` stamps a new knowledge-context pack into a directory.

It intentionally does **not** ship a graph-database backend, require an external SDK,
or default to any embedder — a Space must declare one. (It *is* a Rust library; the
"no SDK" point means no separate SDK package to install, not that there's no API.)

## Quickstart

```sh
just check                                                  # fmt + clippy + test

# read a surface (e.g. a memory dir) into an okf-ext bundle, then validate it
cargo run -- migrate-surface memory ./my-memory --to bundle
cargo run -- validate bundle

# downgrade to canonical OKF (always-allowed target), and visualize the graph
cargo run -- export bundle ./okf-out --target files-only
cargo run -- graph bundle --emit html > graph.html
```

Swap `memory` for `skills`, `devtel`, `container`, or `okf` to read a different
surface.

## How it works

- `okf-ext/0.1` is a superset of OKF: structurally an OKF bundle, with a closed
  vocabulary and typed links. Emitted okf-ext bundles are parsed strictly —
  unknown frontmatter keys are an error. Foreign surfaces (memory, skills, raw
  OKF) are read leniently (`parse_fm_tolerant`): real-world frontmatter with
  unquoted colons is tolerated, and keys outside the vocabulary are dropped on the
  way in. See [`spec/okf-ext-0.1.md`](spec/okf-ext-0.1.md).
- Surface adapters (`src/surface.rs`) map each operator surface into the unified
  note model (`src/models.rs`).
- Export keeps the OKF projection and moves the rest to a `.ckf/` sidecar so import
  is lossless (`src/okf.rs`).
- The pack is a separate process speaking JSON-RPC over stdio (`src/rpc.rs`); it
  brings its own DuckDB store (`src/store.rs`) and stays outside the host's
  dependency boundary.
- The embedder seam (`src/embed.rs`) is explicit-required: there is no default and
  no fallback tokenizer or HTTP endpoint. A Space must set `[seat.knowledge].embedder`
  in its manifest or the knowledge methods refuse to run; `local` declares network
  mode `none`.

## Status

Library + CLI, version `0.1.0` (pre-1.0 SemVer; the `okf-ext/0.1` format and the
`SurfaceAdapter` trait API are not yet stable).

- **Shipped.** The five surface adapters (`memory`, `skills`, `devtel`,
  `container`, `okf`), `migrate-surface`, `export` / `import`, `validate`
  (`E001`–`E004`), `roundtrip`, `graph` (JSON / HTML), and `init`. The JSON-RPC
  transport with `ping`, `shutdown`, and `capabilities` live; embedded DuckDB store.
- **In build.** The knowledge RPC methods (`index_space`, `query`, `export_okf`, …)
  are registered stubs that return `-32000`; wiring them to the store + embedder +
  export is follow-up work (see [`docs/adr/0002`](docs/adr/0002-pack-owns-store-open-dispatch.md)).
- **Not built.** No graph-database backend, no external SDK package, no default
  embedder — a Space must declare one.

## Development

```sh
pre-commit install   # one-time after clone
just check           # the merge gate
```

See `docs/STYLE.md` for documentation tone, `CLAUDE.md` for engineering
conventions, and the ADRs for design decisions:
[`0001`](docs/adr/0001-no-kuzu-graph-backend.md) (why the Kùzu graph backend was
dropped) and [`0002`](docs/adr/0002-pack-owns-store-open-dispatch.md) (own-store +
open dispatch + separate-process AGPL boundary).

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
