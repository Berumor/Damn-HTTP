# Status

Last updated: 2026-10-09

## Current

- Milestone: **Plan** (before M0)
- Task: `project-plan`, branch `feature/project-plan`
- State: PR open, **waiting for maintainer approval**. No application code may be written before it is approved.

## Done

- `develop` and `beta` created from `main` and pushed.
- Project memory written: `ROADMAP.md`, `ARCHITECTURE.md` (crate layout, data model, file format draft, Tauri commands), `DECISIONS.md`, `CONVENTIONS.md`, `NOTES.md`, this file, and `AGENTS.md`.

## In progress

- Nothing besides the open PR.

## Blocked

- Everything after the plan: needs approval of the plan PR and answers to the open questions in `NOTES.md`.
- Local builds: Rust and Node are not installed on the maintainer's machine (see `NOTES.md`).

## Open PRs

- `docs: add project plan and project memory` (`feature/project-plan` → `develop`): PR_URL

## Exact next step

1. Wait for the plan PR to be merged. Apply any requested changes on `feature/project-plan`; if answers change a decision, update `DECISIONS.md` (mark entries `accepted` or supersede them) and `ARCHITECTURE.md`.
2. After the merge: `git switch develop && git pull`, then start M0 with `feature/m0-repo-hygiene`, then `feature/m0-cargo-workspace`, then `feature/m0-ci` (see `ROADMAP.md`). `m0-repo-hygiene` needs no toolchain; the other two do.
