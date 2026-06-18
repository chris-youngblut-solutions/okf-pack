# okf-pack

OKF-compatible knowledge-context format and bidirectional adapter (Rust), packaged as a Spaces pack (the architecture supports hot-swap; the knowledge methods are still stubs).

**Tier**: T1  **Language**: rust  **Test discipline**: pragmatic

This file is **read by every Claude Code session** that opens this repo.
Keep it short, scannable, and load-bearing. Long-form prose belongs in
`docs/`. Personal scratch notes (TODOs, ad-hoc commands, half-formed
ideas) belong in `CLAUDE.local.md` (gitignored).

## What this is

A CLI + library that reads knowledge surfaces (memory, skills, dev.tel,
per-project docs, OKF bundles) into a unified internal format (`okf-ext/0.1`),
converts losslessly to/from Google's OKF, validates and graphs bundles, and
serves a JSON-RPC Spaces pack. It intentionally does NOT ship a graph database
(embedded DuckDB store), require an external SDK, or default to any embedder
(explicit-required). The knowledge RPC methods are currently stubs.

## Build / test / lint

All verbs go through `just`. Never run the underlying tool directly in
docs — that creates two sources of truth.

```sh
just            # list all recipes
just fmt        # auto-format
just lint       # lint (warnings → errors)
just test       # run tests per discipline (pragmatic)
just build      # build release artifact
just check      # fmt + lint + test (the merge gate)
```

Tier-2 release path:

```sh
just release patch    # tag + push (CI does the build/sign/SBOM/SLSA)
just release minor
just release major
```

## Architecture

Three to five bullets. Where the code lives, what each top-level dir
does, key invariants a newcomer can't infer from a `tree` listing.
Update when the layout changes.

- `src/` — primary source
- `tests/` — see test-discipline (pragmatic)
- `docs/` — long-form docs; ADRs in `docs/adr/` (`0001` Kùzu dropped, `0002`
  own-store + open RPC dispatch + separate-process AGPL boundary)
- `.github/workflows/` — CI; SHA-pinned actions

The RPC dispatch table (`src/rpc.rs`) is an open `HashMap` keyed by method name —
methods are registered, not match-armed, so a host can hot-swap the pack; see ADR
`0002`. The pack owns its DuckDB store (`src/store.rs`) and runs as a separate
process. Foreign surfaces are read with a lenient frontmatter parser
(`parse_fm_tolerant`); emitted okf-ext is parsed strictly.

## Conventions

- **Conventional Commits**, signed (`git commit -sS`).
- **Lockfile committed**.
- **Pre-commit installed** — `pre-commit install` once after clone.
- **GitHub Actions pinned by SHA** — Renovate / Dependabot keeps fresh.
- **Container base images pinned by digest** if any.
- **Structured logging** — no `print()` / `console.log` in committed code.

## License

See LICENSE files at repo root. Default scaffolding is Apache-2.0 OR MIT
dual (apache-mit-dual).

## See also

- `CLAUDE.local.md` (gitignored) — your scratch notes for this repo.
- `docs/STYLE.md` — documentation tone for `README.md` and `docs/`.
- `docs/adr/` — Architecture Decision Records.
- `~/.claude/CLAUDE.md` (user-scope) — global invariants that apply
  across every repo.
- `~/Documents/coding-standard.md` — the standard this repo conforms to.
