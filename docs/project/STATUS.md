# Status

Last updated: 2026-10-13

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `m1-http-engine`, branch `feature/m1-http-engine`
- State: PR open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6).
- `m1-core-model` (PR #7), `m1-file-format` (PR #8), `m1-workspace-store` (PR #9), `m1-path-variables` (PR #10): all merged.
- `path-variable-segment-names` (PR #11, merged): a path variable is a whole path segment, as in Postman; names may contain `.` and `-` (D-040, accepted by the merge).
- `m1-http-engine` (PR open): `damnhttp-http` sends requests with reqwest + rustls: all body kinds, redirects with the chain recorded, timeout, proxy, custom CA, skip TLS verification, cancellation, response body streamed to a file, sizes and timing, typed errors without secrets. 29 tests against a local HTTP / HTTPS server (D-041).

## What is stubbed or missing in `core`

- `{{var}}` interpolation: references are located but not resolved (`m2-variable-engine`). References inside path variable values are not percent-encoded yet (see `NOTES.md`).
- The store does not handle environments (`m2-environments`) or secrets (`m2-secrets`). Auth inheritance is not resolved yet (`m2-auth`).
- Duplicate-id repair picks the copy by path order; the git-aware rule comes with the git engine (M3).
- No file watching: the tree is read on demand (planned for M3).
- A file with a newer `version` is refused with an error; showing it read-only in the UI comes with the UI tasks.

`core` and `http` have everything M1 needs from them, but nothing connects them yet: no code turns a `Request` into an `HttpRequest` (`m1-send-and-response`). `import` and `git` are empty and there is no `app` crate or frontend.

## What is limited in `http`

- No DNS / connect / TLS timing breakdown; no connection reuse; no zstd (D-041).
- No client certificates, no cookies jar, no HTTP/3.

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `feat: add the HTTP engine` (`feature/m1-http-engine` → `develop`): https://github.com/Berumor/Damn-HTTP/pull/12

## Exact next step

1. Confirm the PR is merged, then `git switch develop && git pull`.
2. `feature/m1-tauri-skeleton`: the `app` crate (Tauri 2), Vite + React + TypeScript strict + Zustand, tauri-specta bindings, restrictive capabilities, app identifier in one place, theme and i18n scaffolding, ESLint, Vitest, frontend CI jobs. Tauri pulls in MPL-2.0 crates: review and extend `deny.toml` there (D-031).
3. Then `feature/m1-packaging-check` (AppImage, dmg, NSIS build-only workflow; watch `aws-lc-sys` on Windows and macOS).
4. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only).
