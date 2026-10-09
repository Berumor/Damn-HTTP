# Status

Last updated: 2026-10-09

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `m1-core-model`, branch `feature/m1-core-model`
- State: PR open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6). CI ran green on PR #6.
- `m1-core-model` (PR open): the data model in `damnhttp-core`: ids, order keys, requests, auth, variables, environments, collections, validation, the `{{name}}` reference rule (D-024) and the committed / local split (D-021, D-033). 29 unit tests.

## What is stubbed or missing in `core`

- No serialization: nothing reads or writes files yet (`m1-file-format`).
- No URL parsing: path variable names are not derived from the URL, pasted URLs are not split, and a `?` in `url` is not rejected (`m1-path-variables`).
- `OrderKey` only validates; generating and renormalizing keys is `m1-workspace-store`.
- Secret environment values are not split from committed data yet (`m2-secrets`). Auth inheritance is not resolved yet (`m2-auth`).
- The model has no `serde` derives (D-033).

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `feat: add core data model with committed/local value split` (`feature/m1-core-model` → `develop`): https://github.com/Berumor/Damn-HTTP/pull/7

## Exact next step

1. Confirm the PR is merged, then `git switch develop && git pull`.
2. `feature/m1-file-format`: deterministic YAML emitter and parser for `CommittedRequest`, collection, folder, workspace and environment files, plus `.damnhttp/values.yaml`; round-trip and determinism tests; JSON Schemas in `schemas/`; first `docs/FORMAT.md`. Choose the YAML parser crate there (D-008) and check it with `cargo deny`. The format draft is in `ARCHITECTURE.md`.
3. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only, auto-delete branches); old merged branches on GitHub.
