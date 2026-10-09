# Status

Last updated: 2026-10-09

## Current

- Milestone: **M0** (CI basics and repo hygiene)
- Task: `m0-cargo-workspace`, branch `feature/m0-cargo-workspace`
- State: PR open, waiting for the maintainer.

## Done

- `develop` and `beta` created from `main` and pushed.
- Plan approved and recorded: PRs #1, #2, #3 merged (D-001 to D-029). No open questions.
- `m0-repo-hygiene` merged (PR #4).
- Toolchain installed on the maintainer's machine (Rust 1.99.0, Node 26, pnpm 11, WebKitGTK 4.1, cargo-deny).
- `m0-cargo-workspace` (PR open): Cargo workspace with empty `damnhttp-core`, `-http`, `-import`, `-git` crates, pinned toolchain, workspace lints, `clippy.toml`, `rustfmt.toml`, `deny.toml`. `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` and `cargo deny check` pass locally.

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `chore: add cargo workspace with empty crates` (`feature/m0-cargo-workspace` → `develop`): PR_URL

## Exact next step

1. `feature/m0-ci`, branched from `feature/m0-cargo-workspace` (CI needs the workspace to have something to check). Merge the workspace PR first.
2. After M0: `git switch develop && git pull`, then M1 starts with `feature/m1-core-model` (see `ROADMAP.md`).
