# Status

Last updated: 2026-10-13

## Current

- Milestone: **M1** (model, file format, request editor, send, response viewer)
- Task: `m1-workspace-commands`, branch `feature/m1-workspace-commands`
- State: PR open, waiting for the maintainer.

## Done

- Plan approved and recorded: PRs #1 to #3 (D-001 to D-029). No open questions.
- **M0 complete**: repo hygiene (PR #4), Cargo workspace (PR #5), CI (PR #6).
- M1 so far, all merged: `m1-core-model` (PR #7), `m1-file-format` (PR #8), `m1-workspace-store` (PR #9), `m1-path-variables` (PR #10), `path-variable-segment-names` (PR #11), `m1-http-engine` (PR #12), `m1-tauri-skeleton` (PR #13), `frontend-tooling-defaults` (PR #14), `m1-packaging-check` (PR #15, D-044 approved by the maintainer in the PR).
- `m1-workspace-commands` (PR open): 14 new commands for workspaces and the tree, `AppState`, the allowed-folders rule, the recent list, DTOs, error codes (D-045). 7 new tests in the app crate. No UI change.

## What works for a user

- `pnpm tauri dev` opens a window with the app name, a theme switch (System / Light / Dark) and the version read from the backend. Nothing else.

## What is stubbed or missing

- **The UI does not call the new commands yet.** The window still shows only the placeholder screen (`m1-workspace-ui` is next).
- No commands yet for reading or saving a request, collection or folder, for sending, or for settings; the theme is kept in the webview's local storage.
- No keyboard shortcuts yet (they come with the screens they act on).
- Installers are built in CI but are not published anywhere, and are unsigned.
- In `core`: `{{var}}` interpolation, environments, secrets, auth inheritance (M2); git-aware duplicate-id repair and file watching (M3).
- In `http`: no DNS / connect / TLS timing breakdown, no connection reuse (D-041).

## In progress

- Nothing besides the open PR.

## Blocked

- Nothing.

## Open PRs

- `feat: add workspace and tree commands` (`feature/m1-workspace-commands` → `develop`): https://github.com/Berumor/Damn-HTTP/pull/16

## Exact next step

1. Confirm the PR is merged, then `git switch develop && git pull`.
2. Ask the maintainer for the result of the macOS test planned for 2026-10-14 and record it in `NOTES.md`. Windows stays untested until they have a machine.
   **Still unanswered:** did the AppImage window render correctly on the maintainer's machine? (asked in PR #15 and in chat; see `NOTES.md`, "Platform findings"). Ask again and record the answer.
3. `feature/m1-workspace-ui`: the start screen (create, open, recent) and the sidebar tree with create / rename / duplicate / delete, calling the commands through `ui/src/bindings.ts`. A `workspace` Zustand store holds the open workspace and the tree; reload the tree after every change. Translate `AppError.code` through the i18n catalogue. Component tests with the bindings mocked, as in `ui/src/App.test.tsx`.
4. Then `feature/m1-tree-drag-and-drop`, then `m1-request-editor`.
5. Still open for the maintainer (see `NOTES.md`): repo settings (required checks, merge commits only). The app icon stays a placeholder until the maintainer's designer delivers one.
