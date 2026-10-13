# Status

Last updated: 2026-10-13

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `path-variable-segment-names`, branch `fix/path-variable-segment-names`
- State: PR open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6).
- `m1-core-model` (PR #7), `m1-file-format` (PR #8), `m1-workspace-store` (PR #9), `m1-path-variables` (PR #10): all merged. D-039 accepted by the maintainer in PR #10, except one point.
- `path-variable-segment-names` (PR open): the maintainer's change from PR #10. A path variable is now a whole path segment, as in Postman: `/files/:name.pdf` has the variable `name.pdf`. Name grammar widened to `[A-Za-z_][A-Za-z0-9_.-]*` (D-040). 137 tests in `core`.

## What is stubbed or missing in `core`

- `{{var}}` interpolation: references are located but not resolved (`m2-variable-engine`). References inside path variable values are not percent-encoded yet (see `NOTES.md`).
- The store does not handle environments (`m2-environments`) or secrets (`m2-secrets`). Auth inheritance is not resolved yet (`m2-auth`).
- Duplicate-id repair picks the copy by path order; the git-aware rule comes with the git engine (M3).
- No file watching: the tree is read on demand (planned for M3).
- A file with a newer `version` is refused with an error; showing it read-only in the UI comes with the UI tasks.

`core` has everything M1 needs from it. `http`, `import`, `git` are empty and there is no `app` crate or frontend yet.

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `fix: treat a whole path segment as the path variable name` (`fix/path-variable-segment-names` → `develop`): PR_URL

## Exact next step

1. Confirm the PR is merged, then `git switch develop && git pull`.
2. `feature/m1-http-engine`: the `http` crate (reqwest + rustls, redirects, timeouts, proxy, custom CA, skip TLS verification, timing, sizes, cancellation, body to a temp file), tested against a local server. Decide there how query parameters are encoded when sent (D-039). It needs new dependencies: check each with `cargo deny` and extend `deny.toml` only where needed. It does not depend on the open PR except for `docs/project/`.
3. Then `feature/m1-tauri-skeleton` and `feature/m1-packaging-check`.
4. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only).
