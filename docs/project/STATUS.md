# Status

Last updated: 2026-10-09

## Current

- Milestone: **M0** (CI basics and repo hygiene)
- Task: `plan-answers`, branch `feature/plan-answers`
- State: PR open, waiting for the maintainer. Docs only.

## Done

- `develop` and `beta` created from `main` and pushed.
- `project-plan` merged (PR #1): the plan is approved, application code may start.
- `plan-decisions` merged (PR #2): D-019 to D-023.
- Maintainer's answers from PR #2 recorded: D-022 accepted and amended (UUID ids, first line, duplicate repair), D-024 to D-028 accepted, D-029 proposed (rules module moves to M2). No open questions remain.

## In progress

- Nothing besides the open PR.

## Blocked

- `m0-cargo-workspace`, `m0-ci` and all later code: Rust and Node are not installed on the maintainer's machine (see `NOTES.md`).

## Open PRs

- `docs: record maintainer answers from PR #2` (`feature/plan-answers` → `develop`): PR_URL

## Exact next step

1. `feature/m0-repo-hygiene` (needs no toolchain). It is branched from `feature/plan-answers`, so merge the `plan-answers` PR first.
2. Then `feature/m0-cargo-workspace` and `feature/m0-ci`, both of which need the toolchain installed.
