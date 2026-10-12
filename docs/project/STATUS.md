# Status

Last updated: 2026-10-12

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `m1-file-format`, branch `feature/m1-file-format`
- State: done and pushed; PR to `develop` not opened yet.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6). CI ran green on PR #6.
- `m1-core-model` (PR #7, merged): the data model in `damnhttp-core` (D-021, D-024, D-033).
- `m1-file-format` (branch pushed): `core::format` reads and writes workspace, collection, folder, request and environment files and `.damnhttp/values.yaml`. Own deterministic emitter, `saphyr-parser` for reading (D-034 to D-036), strict reading with line numbers, JSON Schemas in `schemas/v1/` (D-037), first `docs/FORMAT.md`. 66 tests in `core`, including canonical fixtures, round trips, determinism, the D-021 invariant, 20 000 generated strings through writer and parser, schema validation and a check that the spec's examples are canonical.

## What is stubbed or missing in `core`

- No filesystem code: nothing walks a workspace, names files or writes to disk yet (`m1-workspace-store`). The format functions work on text.
- No URL parsing: path variable names are not derived from the URL, pasted URLs are not split, and a `?` in `url` is not rejected (`m1-path-variables`).
- `OrderKey` only validates; generating and renormalizing keys is `m1-workspace-store`.
- Secret environment values: the writer drops them, but there is no type for the split yet and no `secrets.yaml` (`m2-secrets`). Auth inheritance is not resolved yet (`m2-auth`).
- A file with a newer `version` is refused with an error; showing it read-only in the UI comes with the UI tasks.

## In progress

- Nothing.

## Blocked

- Nothing.

## Open PRs

- None yet. `feature/m1-file-format` is pushed. PR title: `feat: add the v1 file format with reader, writer, schemas and spec`.

## Exact next step

1. Open the PR for `feature/m1-file-format` → `develop` (if not yet done) and record its link here. Wait for the maintainer to review D-034 to D-037 and merge.
2. Then `git switch develop && git pull` and start `feature/m1-workspace-store` (see `ROADMAP.md`). It does not depend on the UI; it does depend on this PR.
3. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only, auto-delete branches); old merged branches on GitHub.
