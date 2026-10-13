# Status

Last updated: 2026-10-13

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `m1-packaging-check`, branch `feature/m1-packaging-check`
- State: PR #15 open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6).
- M1 so far, all merged: `m1-core-model` (PR #7), `m1-file-format` (PR #8), `m1-workspace-store` (PR #9), `m1-path-variables` (PR #10), `path-variable-segment-names` (PR #11), `m1-http-engine` (PR #12), `m1-tauri-skeleton` (PR #13), `frontend-tooling-defaults` (PR #14, D-043).
- `m1-packaging-check` (PR #15, open): the "Packaging check" workflow builds the AppImage, both macOS `.dmg` files and the Windows NSIS installer without publishing (D-044). First run: see `NOTES.md`, "Platform findings".

## What works for a user

- `pnpm tauri dev` opens a window with the app name, a theme switch (System / Light / Dark) and the version read from the backend. Nothing else.

## What is stubbed or missing

- The UI does not use `core` or `http` yet: no workspace, no tree, no editor, no sending.
- `AppError` exists but no command returns it yet. No settings commands; the theme is kept in the webview's local storage.
- No keyboard shortcuts yet (they come with the screens they act on).
- Installers are built in CI but are not published anywhere, and are unsigned.
- In `core`: `{{var}}` interpolation, environments, secrets, auth inheritance (M2); git-aware duplicate-id repair and file watching (M3).
- In `http`: no DNS / connect / TLS timing breakdown, no connection reuse (D-041).

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `ci: add a build-only packaging check` (`feature/m1-packaging-check` → `develop`): https://github.com/Berumor/Damn-HTTP/pull/15

## Exact next step

1. Confirm PR #15 is merged, then `git switch develop && git pull`.
2. Ask the maintainer whether the AppImage window rendered correctly on CachyOS, and record the answer in `NOTES.md` (it is the one platform finding still open).
3. `feature/m1-workspace-ui`: onboarding ("Create new workspace", "Open local folder") and the sidebar tree with create / rename / duplicate / delete / drag and drop. It is the first task that needs commands wrapping `core::store`; follow "Adding a command" in `CONVENTIONS.md`. It is large: split it (commands and state first, then the tree, then drag and drop).
4. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only). The app icon stays a placeholder until the maintainer's designer delivers one.
