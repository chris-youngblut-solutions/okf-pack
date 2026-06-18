# okf-ext/0.1 — format specification

okf-ext is a superset of Google's Open Knowledge Format (OKF). A bundle is a
directory of Markdown files, one concept per file, each opening with a YAML
frontmatter block. okf-ext keeps OKF's structure and adds a closed vocabulary and
typed links. Canonical pure-OKF is produced only at the export boundary.

The machine-readable schema is `okf-ext-0.1.schema.json` (generated from the
implementation; a drift test keeps them in sync).

## Bundle

A bundle is a directory of `.md` files. Each file is one note. `index.md` is the
directory index (`type: index`). The `.okf/` directory holds the materialized
store and export sidecars and is not part of the note set.

## Frontmatter fields

Fields fall in three tiers.

### Required

- `type` — closed enum (see Type axis).
- `id` — stable identity, independent of path and title.
- `title` — human-readable name.
- `updated` — ISO `YYYY-MM-DD`. Becomes OKF `timestamp` on export.

### Reserved (the OKF projection)

`description`, `resource`, `tags`, `timestamp`, `created`. These carry through to
the OKF note verbatim.

### Extension (okf-ext only; moved to a sidecar on OKF export)

- Closed-vocab axes: `root`, `tier`, `status`, `fde-domain`.
- `container`, `container-path`, `tech`, `hardware`, `stakeholder`, `trigger`,
  `stop-if`, `supersedes`, `superseded-by`.
- Surface extensions (adapter-specific): `x-mem-kind`, `x-origin-session-id`,
  `x-when-to-use`, `x-argument-hint`, `x-allowed-tools`. These map Claude Code
  workspace surfaces (memory, skills) into okf-ext; an adapter for another surface
  may define different `x-*` fields. They are namespaced `x-` so they never collide
  with the reserved OKF projection.
- `links` — typed links (see below).

Unknown frontmatter keys are rejected: a misspelled field is a hard error, not a
silently-dropped value. Extending the vocabulary is a deliberate change to the
schema, not an ad-hoc addition.

### Example

A note with one field from each tier — required, reserved, and extension:

```markdown
---
type: project              # required
id: sample-project         # required (rename-safe)
title: Sample Project      # required
updated: 2026-06-17        # required → OKF `timestamp` on export
description: A one-line description of the concept.   # reserved (OKF passthrough)
tier: t1                   # extension (→ sidecar on OKF export)
links:                     # extension
  - id: another-note
    rel: depends-on
    note: a related note
---

Body Markdown here, with an inline [[other-note]] wikilink.
```

## Type axis (closed)

`project`, `root`, `domain`, `decision`, `fork`, `log`, `reference`, `memory`,
`skill`, `adr`, `scope`, `task`, `index`, `doc`.

`decision` (cross-cutting, globally identified) and `adr` (per-project, numbered)
are distinct.

## Typed links

`links` is a list of `{id, rel, note?}`. `rel` is one of `related`, `depends-on`,
`supersedes`, `superseded-by`, `mentions`. The optional `note` is a human-readable
label for the link (e.g. "reference implementation"); it does not affect link
semantics and is dropped on OKF export. `mentions` is derived from inline
`[[wikilinks]]` and is not hand-authored.

On OKF export each link is emitted as a plain Markdown link under a generated
`## Related` section — the predicate survives as the link text — and is recorded
structurally in `.ckf/links.json`.

## OKF export and import

`okf-pack export` produces a pure-OKF bundle:

- the required + reserved fields become the OKF note;
- the extension layer moves to `.ckf/sidecar/<id>.yaml`;
- typed links degrade as described above;
- `.ckf/typemap.json` records the original type, `ckf.toml` records the profile,
  item count, and a sha256 over the emitted notes.

`okf-pack import` reverses this losslessly when the sidecar is present (it merges
the OKF projection with the extension sidecar and strips the generated `## Related`
section), and loosely for a foreign OKF bundle (id taken from the filename;
extension fields absent).

## Privilege gate

`export --target okf-gcp` refuses any note matching the workspace denylist
(`~/.claude/ship-prep/denylist.md`); `export --target files-only` is always
allowed. The same gate applies to any external embedder, so privileged content
never leaves the machine by either path.
