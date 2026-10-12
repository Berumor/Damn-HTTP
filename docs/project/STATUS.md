# Status

Last updated: 2026-10-12

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `m1-workspace-store`, branch `feature/m1-workspace-store`
- State: PR open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6).
- `m1-core-model` (PR #7, merged): the data model (D-021, D-024, D-033).
- `m1-file-format` (PR #8, merged): `core::format`, JSON Schemas, `docs/FORMAT.md` (D-034 to D-037, accepted by the merge).
- `m1-workspace-store` (PR open): `core::store::WorkspaceStore` (create, open, tree, create / read / save / rename / duplicate / move / delete, local values in `.damnhttp/values.yaml`, `.gitignore` entry, "clear unused local data", duplicate-id repair), order key generation in `model::order`, slugs. 113 tests in `core`, the store's on temporary directories (D-038).

## What is stubbed or missing in `core`

- No URL parsing: path variable names are not derived from the URL, pasted URLs are not split, and a `?` in `url` is not rejected (`m1-path-variables`).
- The store does not handle environments (`m2-environments`) or secrets (`m2-secrets`). Auth inheritance is not resolved yet (`m2-auth`).
- Duplicate-id repair picks the copy by path order; the git-aware rule comes with the git engine (M3).
- No file watching: the tree is read on demand (planned for M3).
- A file with a newer `version` is refused with an error; showing it read-only in the UI comes with the UI tasks.

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `feat: add the workspace store` (`feature/m1-workspace-store` → `develop`): PR_URL

## Exact next step

1. Confirm the PR is merged, then `git switch develop && git pull`.
2. Next tasks do not depend on each other and can go in either order: `feature/m1-path-variables` (the shared URL parser in `core`, D-028) and `feature/m1-http-engine` (`http` crate). Start with `m1-path-variables`.
3. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only, auto-delete branches); old merged branches on GitHub.
