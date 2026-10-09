# Conventions

## Git and merge workflow

- Long-lived branches: `main` (stable releases), `beta` (pre-releases), `develop` (integration). Never commit or push to them directly.
- Short-lived branches: `feature/<task>` and `fix/<task>` from an up-to-date `develop`. `<task>` is a kebab-case slug and matches the task slug in `ROADMAP.md` (e.g. `feature/m1-file-format`). One branch = one focused task.
- Commits: Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`, `refactor:`, `ci:`; `!` or a `BREAKING CHANGE:` footer for breaking changes), signed off with `git commit -s`. Commit often.
- PRs target `develop`. The title is a valid Conventional Commit message (it becomes the squash commit). The description has: what changed, why, how it was tested, screenshots for UI changes, decisions recorded in `DECISIONS.md`.
- The app builds and tests pass at every PR. Docs in `docs/project/` are updated in the same PR.
- Agents never merge. After opening a PR, continue only with work that does not depend on it. After a merge, update local `develop` and branch from there.
- If `gh` is unavailable: push the branch, write the PR title and description into `STATUS.md`, tell the maintainer.
- Never force-push a shared branch or rewrite pushed history.

Merge strategy:

| From → to | Method |
|---|---|
| `feature/*`, `fix/*` → `develop` | squash |
| `develop` → `beta` | merge commit, never squash |
| `beta` → `main` | merge commit, never squash |
| back-merges `main` → `beta`, `main` → `develop` | merge commit |
| hotfix `fix/*` → `main` | squash, then back-merge down the chain |

Promotion PRs are opened by the maintainer. Nothing reaches `main` without having been a beta.

## Rust

- Stable toolchain pinned in `rust-toolchain.toml`. Edition 2021 or newer, set once in the workspace.
- `cargo fmt` default style. `cargo clippy --workspace --all-targets -- -D warnings`. Lints are configured once under `[workspace.lints]`.
- Dependencies and their versions are declared in `[workspace.dependencies]`; crates use `workspace = true`.
- `unsafe` is forbidden (`#![forbid(unsafe_code)]`) in every crate.
- No `unwrap()` / `expect()` / `panic!` outside tests, except for invariants that are proven in a comment.
- `core` and `http` must not depend on Tauri or on anything UI-related.
- Naming: packages `damnhttp-<name>`; modules and functions `snake_case`; Tauri commands `<area>_<verb>` (`request_save`).

## Error handling

- Library crates define their own error enum with `thiserror`. No `anyhow` in library crates.
- `app` converts every error into `AppError { code, message, details }`, where `code` is a stable `SCREAMING_SNAKE_CASE` string. The frontend shows text from the i18n catalogue keyed by `code`; `message` is English diagnostic text for logs and the Advanced view.
- Git errors keep the captured stderr in `details`. It is shown only in Advanced.
- Secret values never appear in errors, logs or events.

## TypeScript / React

- `strict: true`, plus `noUncheckedIndexedAccess`. No `any`; no `@ts-ignore` without a comment explaining why.
- ESLint + Prettier. Function components and hooks only.
- The frontend calls Rust only through `ui/bindings.ts` (generated; never edited by hand). No direct `invoke("...")` with string names.
- State in Zustand stores, one per area. Components do not hold server state of their own.
- Every user-visible string goes through i18n. Simple-mode strings use no git vocabulary (see the brief for the wording).
- Colors and spacing come from CSS variables; no hard-coded colors in components.

## Tests

- Rust unit tests live next to the code (`#[cfg(test)]`); cross-module tests in `crates/<name>/tests/`. Fixtures in `crates/<name>/tests/fixtures/`.
- Serialization: every model type has a round-trip test and a determinism test (serialize twice, compare bytes; parse then serialize, compare with the source file).
- Filesystem and git tests use temp dirs and temp repos. No test touches the network; HTTP tests use a local server, git tests use `file://` remotes.
- Importer fixtures must be redistributable; each fixture directory has a `SOURCE.md` naming origin and license.
- Secret-leak rules: each rule has positive and negative cases; false positives are regressions.
- Frontend: Vitest + Testing Library next to the component (`*.test.tsx`); one Playwright smoke test in `e2e/`.

## Run, build, test

Nothing is scaffolded yet. These are the commands the scaffold will support; correct this section in the PR that introduces each one.

| Purpose | Command | Available from |
|---|---|---|
| Format check | `cargo fmt --all --check` | `m0-cargo-workspace` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | `m0-cargo-workspace` |
| Rust tests | `cargo test --workspace` | `m0-cargo-workspace` |
| License / advisory check | `cargo deny check` | `m0-cargo-workspace` |
| Install frontend deps | `pnpm install` | `m1-tauri-skeleton` |
| Run the app | `pnpm tauri dev` | `m1-tauri-skeleton` |
| Frontend checks | `pnpm lint`, `pnpm typecheck`, `pnpm test` | `m1-tauri-skeleton` |
| Build installers | `pnpm tauri build` | `m1-tauri-skeleton` |
| Smoke test | `pnpm e2e` | `m1-smoke-test` |

## Docs

- `docs/project/`: project memory, short and factual.
- `docs/`: user and contributor docs (`FORMAT.md`, `GIT_AUTH.md`, `RELEASING.md`, guides).
