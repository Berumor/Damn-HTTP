# Status

Last updated: 2026-10-09

## Current

- Milestone: **M0** (CI basics and repo hygiene)
- Task: `m0-repo-hygiene`, branch `feature/m0-repo-hygiene` (branched from `feature/plan-answers`)
- State: PR open, waiting for the maintainer.

## Done

- `develop` and `beta` created from `main` and pushed.
- `project-plan` merged (PR #1): the plan is approved, application code may start.
- `plan-decisions` merged (PR #2): D-019 to D-023.
- `plan-answers` (PR #3, open): D-022 amended, D-024 to D-029. No open questions remain.
- `m0-repo-hygiene` (PR open): `NOTICE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, README, issue forms (bug, feature), PR template, `.editorconfig`, `.gitignore` entries. Issue forms were not validated locally; check that "New issue" on GitHub shows them after the merge.

## In progress

- Nothing besides the open PRs.

## Blocked

- `m0-cargo-workspace`, `m0-ci` and all later code: Rust and Node are not installed on the maintainer's machine (see `NOTES.md`). **Nothing else can be done until they are.**

## Open PRs

- `docs: record maintainer answers from PR #2` (`feature/plan-answers` → `develop`): https://github.com/Berumor/Damn-HTTP/pull/3
- `docs: add repo hygiene files and templates` (`feature/m0-repo-hygiene` → `develop`): PR_URL. Contains the commits of PR #3; merge #3 first.

## Exact next step

1. Confirm both PRs are merged, then `git switch develop && git pull`.
2. Check the toolchain: `cargo --version`, `node --version`, `pnpm --version`. If missing, stop and ask the maintainer (install command in `NOTES.md`).
3. `feature/m0-cargo-workspace`, then `feature/m0-ci` (see `ROADMAP.md`).
