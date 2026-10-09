# Project brief: Damn HTTP, an open-source, git-native REST API client (Tauri + Rust)

> This file is the original brief and lives at `docs/project/BRIEF.md`. Do not rewrite it. If a decision changes, record it in `docs/project/DECISIONS.md` and tell me.

## Goal
Build **Damn HTTP**, a cross-platform desktop app (Windows, macOS, Linux) that is an open-source alternative to Postman, Insomnia, Bruno and Altair, **licensed Apache-2.0**.

The differentiator is **git-based team sync that non-developers can use**. A team shares and updates API collections through a git repository, without passing around docs, files or YAML by hand. The git layer must be usable by people who have never used git.

Scope for v1: **REST only** (no GraphQL, gRPC, WebSocket).

The git repository is already initialized, contains this brief as its first commit on `main`, and is the current working directory. Scaffold the project inside it. Do not create a new repo.

## Stack
- Tauri 2, Rust backend, **React + Vite + TypeScript (strict)** frontend, Zustand for state.
- Code/JSON editor: CodeMirror 6 (NOT Monaco). The webview on Linux is WebKitGTK, so keep the UI light and test there early.
- All HTTP requests are sent from the Rust backend (`reqwest` + `rustls`), never from the webview. Support redirects, timeouts, proxy settings, custom CA and "skip TLS verification" per request, and return timing, size, headers and body.
- Git: use `git2` (or `gix`) for local operations (init, status, diff, commit, log, branches), so the app works with no install. **All network operations (clone, fetch, pull, push) go through the system `git` executable**, so the user's existing credentials, SSH agent, credential helpers and signing config are used as is. Hide both behind a `GitEngine` trait. If system git is not found, the app still works in local-only mode.
- The app never stores or asks for git credentials itself: no OAuth, no token fields, no custom keychain entries for git. (The `keyring` crate is still used for non-git secrets if needed.)
- App identifier: use the placeholder `com.damnhttp.app` and keep it in a single config location so it is easy to change.

## Core features (v1)
1. **Collections**: workspace > folders > requests, with drag and drop, rename, duplicate.
2. **Requests**: method, URL, query params (table, enable/disable per row), headers, body (none, JSON, text, form-urlencoded, multipart), **path variables using the `:id` syntax** (same as Postman and Bruno; e.g. `/users/:id`, with a table to set values), description.
3. **Variables**: collection-level and environment-level (e.g. dev/staging/prod), `{{var}}` interpolation in URL, headers, params, body and auth, with highlighting and autocomplete in the UI. Show the resolved value on hover. Path variables (`:id`) and `{{var}}` are separate mechanisms and must not conflict (e.g. a port like `localhost:8080` is not a path variable).
4. **Secrets**: environment values flagged as secret are stored ONLY in a git-ignored local file or the OS keychain, never in committed files. The UI must make it visible which values are secret and that they are not synced. A teammate who clones the repo sees the secret keys as empty fields to fill in.
5. **Secret-leak guard**: before "Save version" (commit), scan the changes for values in non-secret fields that look like credentials (e.g. `Bearer eyJ...`, JWTs, long high-entropy strings, `Authorization`/`api-key`/`token`/`password` fields with literal values, common key prefixes such as `ghp_`, `AKIA`, `sk-`). Warn the user and offer one click to move the value into a secret variable. Never block hard; allow "save anyway". Keep the detection rules in one documented module with tests, and keep false positives low.
6. **Auth** (for the requests being tested, not for git): None, Basic, Bearer (collection-level default, overridable per folder/request, with inheritance). Design it so adding OAuth2/API key later is easy.
7. **Response viewer**: status, time, size, headers, pretty/raw/preview, JSON folding, copy, save to file, search. Must handle large responses (several MB) without freezing.
8. **History**: local only, never committed.
9. **Import** (with a preview and a report of what could not be converted): Postman v2.1 collections and environments, Insomnia v4 export, Bruno collections (.bru), OpenAPI 3.x / Swagger 2.0 (JSON and YAML, local file or URL; generate folders by tag, examples as bodies, servers as environment variables). Convert path variable syntax from other tools to `:id` on import. Export to OpenAPI is a nice-to-have, not v1.

## File format (critical)
- Collections are plain text files in a normal folder, designed to produce clean diffs and rare merge conflicts: **one YAML file per request**, one folder per collection folder, stable key ordering, deterministic serialization (same input = byte-identical output), no timestamps or random IDs that change on every save, IDs only where stable identity is required.
- Use YAML with a versioned schema (`version:` field). Publish it as a real spec in `docs/FORMAT.md`, with examples, and include a JSON Schema for editor validation.
- Secrets and local state live in git-ignored paths (auto-create `.gitignore` entries).

## Git UX for non-developers (the main focus)
- Do not show git vocabulary by default: "Sync" (commit + pull --rebase + push in one action), "Save version" (commit), "Updates from your team" (incoming changes), "Your changes" (diff). Keep an "Advanced" toggle that exposes branches, commit log, and raw git status.
- Onboarding: "Open workspace from a Git URL", "Create new workspace", "Open local folder".
- **Authentication: delegated entirely to system git.** Run git commands non-interactively (set `GIT_TERMINAL_PROMPT=0`, never block waiting on a terminal prompt, always use timeouts) and capture stderr.
- **Git detection:** on startup, check for `git` and its version. If missing, show a friendly screen explaining that syncing needs Git, with a download link per OS and a "Continue without sync" option. Local-only mode keeps working.
- **Connection check:** when opening or cloning a remote, run a lightweight check (`git ls-remote`) and map failures to friendly messages: "Git couldn't sign in to this repository. Sign in once with your usual Git tool (or see our guide), then try again." Link to `docs/GIT_AUTH.md`.
- Write `docs/GIT_AUTH.md` covering the common setups: Git Credential Manager (Windows/macOS), the macOS keychain helper, SSH keys and agent, and GitHub CLI (`gh auth login`) as the easiest path on Linux.
- Background fetch every N minutes with a non-intrusive badge ("3 updates from your team").
- Diff view at the request level (changed URL, headers, params, body), not raw text, for non-devs; a raw diff in Advanced.
- Conflict resolution UI at the field level (keep mine / keep theirs / compare). Never show `<<<<<<<` markers to the user. Never force-push from the simple UI.
- Branch support: simple mode works on the default branch; Advanced allows creating/switching branches.
- Clear, friendly error messages for auth failure, offline, non-fast-forward, and dirty state.

## Non-goals for v1
Scripting (pre-request/test scripts), GraphQL/gRPC/WebSocket, cloud accounts, telemetry, mock servers, collaboration outside git, OAuth, in-app token management, GitHub/GitLab-specific integrations, code signing/notarization, in-app auto-updater, Flatpak/deb/rpm/Snap packages, ARM Linux builds, a CLI (but see the architecture note below). Do not add telemetry of any kind.

## Architecture & quality requirements
- Cargo workspace with separate crates: `core` (models, serialization, variable resolution, secret-leak detection; **no Tauri dependency**), `http` (request execution; **no Tauri dependency**), `import` (one module per source format), `git` (`GitEngine` trait + implementations), `app` (Tauri commands). Keep `core` and `http` free of UI concerns so a future `damnhttp run` CLI (execute a collection in CI) can be added as another crate with little work. The frontend talks to Rust only through typed Tauri commands; generate TS types from Rust (e.g. `specta`/`tauri-specta` or `ts-rs`).
- Tauri capability/permission config as restrictive as possible; no arbitrary filesystem or shell access from the webview.
- Tests: unit tests for serialization round-trips (including determinism), variable and path-variable interpolation, secret-leak detection, each importer, git flows against temporary repos (clone, edit, sync, conflict). Frontend: component tests for the key screens and one Playwright smoke test.
- **Importer fixtures:** importers fail on messy real-world files, not clean examples. Build fixtures from realistic, messy exports (large collections, odd auth setups, nested folders, missing fields, big OpenAPI specs with `$ref`s, Swagger 2.0 quirks). Only use material whose license allows redistribution. Where you cannot get real exports, write realistic ones, and list in `NOTES.md` which real exports I should supply.
- CI (GitHub Actions), running on every pull request to `develop`, `beta` and `main` and on pushes to those branches: Rust fmt, clippy (deny warnings), tests, `cargo-deny` (everything must be compatible with Apache-2.0), frontend lint + typecheck + tests, PR title check (Conventional Commits), DCO sign-off check. Cache dependencies.
- i18n-ready UI strings (English first). Keyboard shortcuts for send, new request, sync. Dark/light theme.
- Repo hygiene: `LICENSE` (Apache-2.0), `NOTICE`, `CONTRIBUTING.md` (document DCO sign-off with `git commit -s`; no CLA), `CODE_OF_CONDUCT.md`, `SECURITY.md`, issue templates (bug, feature) and a PR template, `README.md` (name: Damn HTTP), `docs/`.
- **Platform testing:** build the Linux AppImage early (see M1) and note any AppImage/WebKitGTK problems, especially on rolling distros (Arch/CachyOS), in `NOTES.md`.

## Project memory: the `docs/project/` directory
Other agent sessions will continue this work with no memory of this one, so `docs/project/` is the single source of truth about the project's state. Create it in the first session and keep it current.

Files (all Markdown, short, factual, no filler):
- `BRIEF.md`: this brief (already present; do not rewrite).
- `STATUS.md`: current milestone, current task and its branch, what is done, what is in progress, what is blocked, open PRs, and the **exact next step** to resume from.
- `ROADMAP.md`: milestones M0-M7 with a checklist of tasks and their state.
- `ARCHITECTURE.md`: crate layout, data flow, main modules and their responsibilities, list of Tauri commands, and how frontend and backend talk. Update it whenever the structure changes.
- `DECISIONS.md`: a log of decisions (ADR style: date, context, decision, alternatives considered, consequences). Record every non-obvious choice, including the ones I made in this brief.
- `CONVENTIONS.md`: code style, naming, error handling, test conventions, git and merge workflow, how to run/build/test the project.
- `NOTES.md`: gotchas, known issues, platform-specific findings (especially Linux/WebKitGTK), open questions for me, and things I need to do outside the repo.

Rules:
- At the **start of every session**: read `AGENTS.md` and everything in `docs/project/` before doing anything else.
- At the **end of every task** (and before opening a PR): update `STATUS.md`, `ROADMAP.md`, and any other file affected. Docs changes ship in the same PR as the code they describe.
- Create an `AGENTS.md` at the repo root (short) that tells any agent to read `docs/project/` first and summarizes the git workflow below.
- Other docs (`docs/FORMAT.md`, `docs/GIT_AUTH.md`, `docs/RELEASING.md`, user guides) stay in `docs/`, outside `docs/project/`.

## Git workflow (git flow with a beta channel)
- Long-lived branches: `main` (stable releases), `beta` (pre-releases, what the team tests), `develop` (integration). Short-lived branches: `feature/<task>` and `fix/<task>`, branched from `develop`. `<task>` is a short kebab-case slug, e.g. `feature/m1-request-editor`, `fix/path-variable-parsing`.
- If `develop` or `beta` do not exist, create them from `main` and push them before anything else.
- **Never commit or push directly to `main`, `beta` or `develop`.** All work happens on a feature/fix branch. (The initial commit with this brief was made by me.)
- One branch = one focused task (a milestone is split into several small tasks and PRs). Keep PRs reviewable.
- Commit often with clear messages in Conventional Commits style (`feat:`, `fix:`, `docs:`, `test:`, `chore:`, `refactor:`, `ci:`; use `!` or a `BREAKING CHANGE:` footer for breaking changes), signed off with `git commit -s` (DCO). The app must build and tests must pass at each PR.
- When a task is done: push the branch and **open a pull request targeting `develop`** (e.g. `gh pr create --base develop`). The PR title must itself be a valid Conventional Commit message, because it becomes the squash commit. The PR description must include: what changed, why, how it was tested, screenshots for UI changes, and any decisions recorded in `DECISIONS.md`. If the GitHub CLI is unavailable or not authenticated, push the branch and write the PR title and description to `docs/project/STATUS.md`, then tell me.
- **Never merge PRs yourself.** I review and merge. After opening a PR, continue only with work that does not depend on it; if the next task depends on it, stop and tell me. Once it is merged, update local `develop` and branch from there.
- **Merge strategy** (document in `CONVENTIONS.md`): **squash-merge** `feature/*` and `fix/*` into `develop`; **merge commits (never squash)** for `develop` → `beta` and `beta` → `main`, so the commit history the release tooling reads stays intact.
- **Promotion flow:** `develop` → `beta` (I open the PR; merging publishes a beta pre-release such as `0.3.0-beta.1`) → `main` (I open the PR; merging publishes the stable release such as `0.3.0`). Nothing reaches `main` without having been a beta first. After a stable release, open back-merge PRs `main` → `beta` and `main` → `develop` so the branches do not drift.
- **Hotfixes:** branch `fix/<task>` from `main`, PR into `main`, then back-merge down the chain.
- Never force-push shared branches, never rewrite history that has been pushed.

## Release & versioning (milestone M7)
- **Semantic Versioning, fully automated from Conventional Commits** using **semantic-release** with branch channels: `main` is the stable channel and `beta` is a prerelease channel (`prerelease: true`). `develop` publishes nothing. Start at `0.1.0` and stay on `0.x` until I decide on 1.0. Do **not** use release-please (it does not handle prereleases well).
- **The git tag is the single source of truth for the version.** Do not commit version bumps back to the branches (no `@semantic-release/git`). The version fields in the repo (`Cargo.toml` workspace version, `tauri.conf.json`, `package.json`) stay at a placeholder (e.g. `0.0.0`); CI stamps the real version into them at build time. Add a CI check that the stamping step works. Record this in `DECISIONS.md`.
- **Release workflow design:** one workflow, triggered by pushes to `beta` and `main`, with this shape: (1) compute the next version with semantic-release in dry-run mode and output it; (2) a build matrix stamps that version and builds the installers with `tauri-apps/tauri-action`, uploading them as workflow artifacts; (3) a final job runs semantic-release for real to create the tag and GitHub Release (marked pre-release on `beta`) and attaches the artifacts plus `SHA256SUMS`. Everything stays in one workflow because tags created with `GITHUB_TOKEN` do not trigger other workflows. Make sure the version built is the version published, and verify the setup works end to end on `beta` before relying on it. Skip the release when there are no releasable commits.
- Build matrix:
  - Linux (x86_64): **AppImage only** for now. Build on `ubuntu-22.04` for glibc compatibility; install the WebKitGTK/Tauri system dependencies listed in the Tauri docs.
  - macOS: Apple Silicon (aarch64) and Intel (x86_64), `.dmg`.
  - Windows (x86_64): NSIS installer `.exe` (add `.msi` only if it is trivial).
- Also run the same matrix **build-only** (no publish) on pull requests to `develop` that touch app code or build config, or at least on a nightly schedule, so packaging breaks are caught early.
- Release notes come from GitHub Releases. If I later want an in-repo `CHANGELOG.md`, generate it with `git-cliff` at release time, without committing version bumps.
- Binaries are **unsigned** in v1. Document in the README how users open them (macOS Gatekeeper, Windows SmartScreen) and the AppImage requirements (`chmod +x`, FUSE/libfuse2 on some distros).
- Write `docs/RELEASING.md`: the full release process step by step (promotion PRs, back-merges, hotfixes), and the repo settings I need to enable (branch protection on `main`/`beta`/`develop`, allowed merge methods, workflow permissions).

## How to work
1. **First session:** read this brief, create `develop` and `beta` if missing, then on a `feature/project-plan` branch produce a short plan: crate layout, file format spec draft, data model, list of Tauri commands, and the task breakdown for M0-M2. Write it into `docs/project/` (ROADMAP, ARCHITECTURE, DECISIONS, CONVENTIONS, STATUS, NOTES) and create `AGENTS.md`. Open a PR to `develop` and **wait for my approval before writing application code**.
2. Then build in milestones, each split into small tasks on their own branches and PRs, keeping the app runnable:
   - **M0:** CI basics (fmt, clippy, tests, cargo-deny, PR title check, DCO check), repo hygiene files, issue/PR templates.
   - **M1:** workspace/collection model + file format + request editor + send request + response viewer. Include in this milestone the Tauri skeleton and a **build-only packaging check on Linux (AppImage), macOS and Windows**, so packaging problems show up early.
   - **M2:** variables, environments, secrets, Basic/Bearer auth.
   - **M3:** git engine (local ops via git2, network ops via system git, git detection, connection check) + simple Sync UI + request-level diff view. **First task of this milestone: a thin end-to-end slice** (open a workspace from a Git URL, edit a request, Save version, Sync, see the change on a second clone). When it works, **stop and tell me**, so I can test it with a non-developer teammate before you build the rest of M3.
   - **M4:** conflict resolution + background fetch + friendly git error mapping + secret-leak guard + `docs/GIT_AUTH.md`.
   - **M5:** importers (Postman, Insomnia, Bruno, OpenAPI/Swagger) with report, using the messy fixtures described above.
   - **M6:** polish, UX review, docs, i18n/theme cleanup, FORMAT.md as a finished spec.
   - **M7:** release pipeline (semantic-release channels, tag-driven versioning, multi-OS builds, `docs/RELEASING.md`).
3. After each task and each milestone, summarize what works, what is stubbed, and any decision you made that I should review (and make sure it is in `docs/project/`).
4. Ask me when a decision is ambiguous and expensive to reverse (file format, git UX flows, release flow); otherwise pick a sensible default and record it in `DECISIONS.md`.
5. Do not buy or register anything (domains, accounts, certificates). If something needs an external account or setting, add it to the "things I need to do" list in `NOTES.md`.
