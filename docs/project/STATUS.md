# Status

Last updated: 2026-10-13

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `frontend-tooling-defaults`, branch `feature/frontend-tooling-defaults`
- State: PR open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6).
- M1 so far, all merged: `m1-core-model` (PR #7), `m1-file-format` (PR #8), `m1-workspace-store` (PR #9), `m1-path-variables` (PR #10), `path-variable-segment-names` (PR #11), `m1-http-engine` (PR #12), `m1-tauri-skeleton` (PR #13, D-042 approved by the maintainer in the PR).
- `frontend-tooling-defaults` (PR #14, open): Prettier without configuration, ESLint with the recommended sets only, Vitest defaults plus `jsdom` and one setup file (D-043).
- What `m1-tauri-skeleton` delivered: the `damnhttp-app` crate (Tauri 2) with one command, `app_info`; deny-by-default command permissions; CSP; tauri-specta bindings with a freshness test; frontend scaffolding (Vite, React 19, TypeScript strict, Zustand, i18next, light / dark theme tokens, ESLint, Prettier, Vitest); placeholder icons; frontend CI job. The app starts and renders on CachyOS (see `NOTES.md`) (D-042).

## What works for a user

- `pnpm tauri dev` opens a window with the app name, a theme switch (System / Light / Dark) and the version read from the backend. Nothing else.

## What is stubbed or missing

- The UI does not use `core` or `http` yet: no workspace, no tree, no editor, no sending.
- `AppError` exists but no command returns it yet. No settings commands; the theme is kept in the webview's local storage.
- No keyboard shortcuts yet (they come with the screens they act on).
- No installers have been built: `m1-packaging-check` is next.
- In `core`: `{{var}}` interpolation, environments, secrets, auth inheritance (M2); git-aware duplicate-id repair and file watching (M3).
- In `http`: no DNS / connect / TLS timing breakdown, no connection reuse (D-041).

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `chore: use default settings for Prettier, ESLint and Vitest` (`feature/frontend-tooling-defaults` → `develop`): https://github.com/Berumor/Damn-HTTP/pull/14

## Exact next step

1. Confirm the PR is merged, then `git switch develop && git pull`.
2. `feature/m1-packaging-check`: a build-only workflow that produces the Linux AppImage (`ubuntu-22.04`), macOS arm64 and x86_64 `.dmg`, and Windows NSIS installer, without publishing. Then run the AppImage on CachyOS and record findings in `NOTES.md`. Watch `aws-lc-sys` on Windows and macOS (D-041).
3. Then `feature/m1-workspace-ui` (onboarding and the sidebar tree): the first task that needs commands wrapping `core::store`.
4. Still open for the maintainer (see `NOTES.md`): repo settings (required checks: add "Frontend (lint, typecheck, tests, build)"; merge commits only). The app icon stays a placeholder until the maintainer's designer delivers one.
