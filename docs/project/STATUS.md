# Status

Last updated: 2026-10-09

## Current

- Milestone: **Plan** (before M0)
- Task: `plan-decisions`, branch `feature/plan-decisions`
- State: PR open, waiting for the maintainer. Docs only.

## Done

- `develop` and `beta` created from `main` and pushed.
- `project-plan` merged (PR #1): the plan is approved, application code may start.
- Maintainer's answers from PR #1 recorded: D-019 (secrets in a git-ignored file only), D-020 (proxy, CA, TLS verification local), D-021 (literal query / path variable values never committed), D-022 (request ids, proposed). `ARCHITECTURE.md` file format draft updated to match.

## In progress

- Nothing besides the open PR.

## Blocked

- `m1-core-model` and later M1 tasks: need answers to the six questions in `NOTES.md` (all about D-021 / D-022).
- `m0-cargo-workspace`, `m0-ci` and all later code: Rust and Node are not installed on the maintainer's machine (see `NOTES.md`).

## Open PRs

- `docs: record plan decisions from PR #1 review` (`feature/plan-decisions` → `develop`): PR_URL

## Exact next step

1. When the `plan-decisions` PR is merged: `git switch develop && git pull`. If the maintainer answered the questions in `NOTES.md`, record the answers first (mark D-022 `accepted` or supersede it, update `ARCHITECTURE.md`).
2. Start M0 with `feature/m0-repo-hygiene` (needs no toolchain, does not depend on the open questions), then `feature/m0-cargo-workspace`, then `feature/m0-ci` (both need the toolchain).
