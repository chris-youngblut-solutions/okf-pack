# justfile — universal verbs.
# Language overlays redefine fmt/lint/test/build with concrete commands.
# Conventions: every verb runnable from a clean clone after `just bootstrap`.

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := true
set allow-duplicate-recipes := true

# Show all recipes when invoked with no arguments.
default:
    @just --list

# One-time setup after clone. Language overlays extend this.
bootstrap:
    @echo "[bootstrap] installing pre-commit hooks"
    @command -v pre-commit >/dev/null 2>&1 || { echo "pre-commit not installed; pipx install pre-commit"; exit 1; }
    pre-commit install --install-hooks
    pre-commit install --hook-type commit-msg

# Auto-format. Overridden by language overlay.
fmt:
    @echo "[fmt] no language overlay loaded — nothing to do"

# Lint. Overridden by language overlay.
lint:
    @echo "[lint] no language overlay loaded — nothing to do"

# Tests. Overridden by language overlay.
test:
    @echo "[test] no language overlay loaded — nothing to do"

# Build release artifact. Overridden by language overlay.
build:
    @echo "[build] no language overlay loaded — nothing to do"

# The merge gate. Whatever fails here blocks PRs.
check: fmt lint test
    @echo "[check] OK"

# Run all pre-commit hooks against the entire repo (not just staged files).
pre-commit-all:
    pre-commit run --all-files

# --- Release verbs ---
# `just release {patch|minor|major}` bumps the version, tags, and pushes.
# CI takes over from the tag push: builds, signs (cosign), generates SBOM
# (syft), and (T2 only) generates SLSA provenance. See
# `.github/workflows/release.yml` for the orchestration.

release bump:
    @bash -euo pipefail -c '\
        bump="{{ bump }}"; \
        case "$bump" in patch|minor|major) ;; *) echo "usage: just release {patch|minor|major}"; exit 2;; esac; \
        if [[ -n "$(git status --porcelain)" ]]; then echo "tree dirty; commit/stash first"; exit 1; fi; \
        if [[ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]]; then echo "release only from main"; exit 1; fi; \
        last=$(git tag --list "v*" --sort=-v:refname | head -n1); \
        if [[ -z "$last" ]]; then last="v0.0.0"; fi; \
        IFS=. read -r maj min pat <<< "${last#v}"; \
        case "$bump" in \
            patch) pat=$((pat+1));; \
            minor) min=$((min+1)); pat=0;; \
            major) maj=$((maj+1)); min=0; pat=0;; \
        esac; \
        new="v${maj}.${min}.${pat}"; \
        echo "[release] tagging $new (was $last)"; \
        git tag -s -a "$new" -m "Release $new"; \
        git push origin "$new"; \
        echo "[release] pushed $new — CI will build, sign, and publish"; \
    '
# Rust justfile overlay.
# Appended to universal justfile.base; redefines fmt/lint/test/build.

# Auto-format Rust source.
fmt:
    cargo fmt --all

# Lint with clippy. Treat warnings as errors.
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run the test suite. Tweaked per test-discipline at /new-project time.
test:
    cargo nextest run --all-features || cargo test --all-features

# Build a release binary.
build:
    cargo build --release --all-features

# Audit dependencies for known advisories.
audit:
    cargo deny check
    cargo audit

# Update + commit Cargo.lock changes only.
update:
    cargo update
    @if [[ -n "$(git status --porcelain Cargo.lock)" ]]; then \
        git add Cargo.lock; \
        git commit -s -m "chore(deps): cargo update"; \
    fi
