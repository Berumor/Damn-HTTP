# Status

Last updated: 2026-10-09

## Current

- Milestone: **M0** (CI basics and repo hygiene)
- Task: `m0-ci`, branch `feature/m0-ci` (branched from `feature/m0-cargo-workspace`)
- State: PR open, waiting for the maintainer. This is the last M0 task.

## Done

- `develop` and `beta` created from `main` and pushed.
- Plan approved and recorded: PRs #1, #2, #3 merged (D-001 to D-029). No open questions.
- `m0-repo-hygiene` merged (PR #4).
- Toolchain installed on the maintainer's machine (Rust 1.99.0, Node 26, pnpm 11, WebKitGTK 4.1, cargo-deny).
- `m0-cargo-workspace` (PR #5, open): Cargo workspace with empty crates, pinned toolchain, lints, `deny.toml`.
- `m0-ci` (PR open): `ci.yml` with Rust, cargo-deny, PR title and commit checks; `.github/scripts/check-commits.sh`.

## In progress

- Nothing besides the open PRs.

## Blocked

- Nothing.

## Open PRs

- `chore: add cargo workspace with empty crates` (`feature/m0-cargo-workspace` → `develop`): https://github.com/Berumor/Damn-HTTP/pull/5
- `ci: add CI workflow with Rust, license, PR title and commit checks` (`feature/m0-ci` → `develop`): PR_URL. Contains the commits of PR #5; merge #5 first.

## Exact next step

1. Confirm PRs #5 and the CI PR are merged, then `git switch develop && git pull`. M0 is then complete: mark it in `ROADMAP.md` in the next PR.
2. Remind the maintainer of the repo settings in `NOTES.md` (required checks, merge commits only, auto-delete branches).
3. Start M1 with `feature/m1-core-model` (see `ROADMAP.md` and the data model in `ARCHITECTURE.md`).
