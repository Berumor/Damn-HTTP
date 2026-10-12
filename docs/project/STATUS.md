# Status

Last updated: 2026-10-12

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `m1-path-variables`, branch `feature/m1-path-variables`
- State: PR open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6).
- `m1-core-model` (PR #7, merged): the data model (D-021, D-024, D-033).
- `m1-file-format` (PR #8, merged): `core::format`, JSON Schemas, `docs/FORMAT.md` (D-034 to D-037).
- `m1-workspace-store` (PR #9, merged): `core::store::WorkspaceStore`, order key generation, slugs (D-038, accepted by the merge).
- `m1-path-variables` (PR open): `core::url` (the shared URL template parser: path variables, references, query split and join, path variable substitution) and `model::url_edit` (URL bar text, path variable table that follows the URL, rename detection). `Request::split` now moves a query string out of `url`; the store aligns path variables on load and save. 135 tests in `core` (D-039).

## What is stubbed or missing in `core`

- `{{var}}` interpolation: references are located but not resolved (`m2-variable-engine`). References inside path variable values are not percent-encoded yet (see `NOTES.md`).
- The store does not handle environments (`m2-environments`) or secrets (`m2-secrets`). Auth inheritance is not resolved yet (`m2-auth`).
- Duplicate-id repair picks the copy by path order; the git-aware rule comes with the git engine (M3).
- No file watching: the tree is read on demand (planned for M3).
- A file with a newer `version` is refused with an error; showing it read-only in the UI comes with the UI tasks.

`core` now has everything M1 needs from it. `http`, `import`, `git` are empty and there is no `app` crate or frontend yet.

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `feat: add the shared URL template parser` (`feature/m1-path-variables` → `develop`): PR_URL

## Exact next step

1. Confirm the PR is merged, then `git switch develop && git pull`.
2. `feature/m1-http-engine`: the `http` crate (reqwest + rustls, redirects, timeouts, proxy, custom CA, skip TLS verification, timing, sizes, cancellation, body to a temp file), tested against a local server. Decide there how query parameters are encoded when sent (D-039). It needs new dependencies: check each with `cargo deny` and extend `deny.toml` only where needed.
3. Then `feature/m1-tauri-skeleton` and `feature/m1-packaging-check`.
4. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only).
