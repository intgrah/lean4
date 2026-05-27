# RIIR: Rust port of the Lean 4 elaborator

This branch (`riir`) is a parallel project. It is not intended to be upstreamed.

## Pinned upstream

The Rust port targets a single, fixed upstream commit:

    leanprover/lean4 @ c462e4333acaffe809774b4d9ed57878342caf92

The Lean source under `src/` at this SHA is the read-only specification. Advancing the pin is a far-future question and out of scope for this work.

## Layout

- `Cargo.toml` (top level) — Cargo workspace manifest.
- `crates/` — Rust crate sources, mirroring the Lean source tree.
- `src/` — Lean source, unchanged from upstream. Read-only spec.

See `/home/intgrah/.claude/plans/write-a-plan-kind-globe.md` for the full plan.
