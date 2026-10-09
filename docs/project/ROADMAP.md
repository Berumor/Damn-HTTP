# Roadmap

One checkbox = one branch = one PR to `develop`. Slugs are the branch names without the `feature/` prefix. M0-M2 are broken down in detail; M3-M7 are outlines that get a detailed breakdown (in a planning PR) when the milestone starts.

Legend: `[ ]` todo, `[~]` in progress or PR open, `[x]` merged.

## Plan

- [x] `project-plan`: project memory in `docs/project/`, `AGENTS.md`. Approved and merged (PR #1).
- [x] `plan-decisions`: maintainer's answers from PR #1 (D-019 to D-022), merge strategy change (D-023). Merged (PR #2).
- [x] `plan-answers`: maintainer's answers from PR #2 (D-022 amended, D-024 to D-029).

## M0: CI basics and repo hygiene

- [x] `m0-repo-hygiene`: `NOTICE`, `CONTRIBUTING.md` (DCO, no CLA), `CODE_OF_CONDUCT.md`, `SECURITY.md`, README expansion, issue templates (bug, feature), PR template, `.editorconfig`, `.gitignore` for Rust + Node. (`LICENSE` already exists.)
- [~] `m0-cargo-workspace`: Cargo workspace with empty `core`, `http`, `import`, `git` crates, `rust-toolchain.toml`, shared lints, `rustfmt.toml`, `deny.toml`.
- [ ] `m0-ci`: GitHub Actions on PRs and pushes to `develop` / `beta` / `main`: fmt, clippy (deny warnings), tests, `cargo-deny`, dependency caching, PR title check (Conventional Commits), Conventional Commits check on every commit of the PR (D-023), DCO sign-off check. Frontend jobs are added in `m1-tauri-skeleton`.

## M1: model, file format, request editor, send, response viewer

- [ ] `m1-core-model`: data model and validation in `core`, including the committed / local split of query and path variable values (D-021) request ids as UUIDs (D-022) and the reference rule (D-024).
- [ ] `m1-file-format`: deterministic YAML emitter + parser, round-trip and determinism tests, a test that no literal query / path value can be serialized into a request file, JSON Schemas in `schemas/`, first `docs/FORMAT.md`.
- [ ] `m1-workspace-store`: on-disk operations in `core`: create/open, tree, CRUD, rename, duplicate, move, ordering, slug collisions, `.gitignore` management, the local values store in `.damnhttp/`, duplicate-id repair on load, "clear unused local data". Tests on temp dirs.
- [ ] `m1-path-variables`: the single shared URL parser in `core` (D-028): path variable names, `{{ }}` skipping, query split, `:id` substitution, rename detection, sparse `path_params` handling. Tests for schemes, ports and the other edge cases.
- [ ] `m1-http-engine`: `http` crate: reqwest + rustls, redirects, timeouts, proxy, custom CA, skip TLS verification, timing, sizes, cancellation, body to temp file. Tests against a local server.
- [ ] `m1-tauri-skeleton`: `app` crate, Tauri 2 + Vite + React + TS strict + Zustand, tauri-specta bindings, restrictive capabilities, app identifier in one place, theme and i18n scaffolding, ESLint, Vitest, frontend CI jobs (lint, typecheck, tests).
- [ ] `m1-packaging-check`: build-only workflow for Linux AppImage (`ubuntu-22.04`), macOS arm64 + x86_64 `.dmg`, Windows NSIS. Run the AppImage on CachyOS and write findings to `NOTES.md`. Done right after the skeleton, before the UI work.
- [ ] `m1-workspace-ui`: onboarding ("Create new workspace", "Open local folder"), sidebar tree with create / rename / duplicate / delete / drag and drop.
- [ ] `m1-request-editor`: method + URL bar (pasted URLs split into committed URL + local values), "not synced" marker on local values with the explanation for non-reference values (D-024), URL highlighting through `url_analyze`, query table, path variable table, headers table, body editors (none, JSON, text, form-urlencoded, multipart), description, save, tabs, shortcuts for send and new request.
- [ ] `m1-send-and-response`: send / cancel wiring, response viewer (status, time, size, headers, pretty / raw / preview, JSON folding, search, copy, save to file), large-body handling.
- [ ] `m1-smoke-test`: Playwright smoke test (create workspace, create request, send to a local server, see the response) wired into CI.

## M2: variables, environments, secrets, auth

- [ ] `m2-variable-engine`: `{{var}}` parser and resolver in `core`: scopes, precedence, nested references, cycle detection, unresolved reporting. Tests for coexistence with path variables.
- [ ] `m2-environments`: environment files and collection variables in the format and store, environment manager UI, active environment selector.
- [ ] `m2-secrets`: `SecretStore` trait with the git-ignored file backend (D-019), secret flag in the UI with a visible "not synced" marker, empty fields for teammates. Tests proving a secret value never reaches a committed file.
- [ ] `m2-variable-editor-ux`: CodeMirror extension: highlight, autocomplete, resolved value on hover (secrets masked), unresolved marker.
- [ ] `m2-detection-rules`: the documented rules module in `core` (D-029): credential patterns and entropy for the leak guard, plus the path-segment hint "Turn this into a path variable?" (D-026), with positive and negative tests.
- [ ] `m2-share-with-team`: "Share with team" and "Make local again" (D-027), using the rules module and a confirmation dialog.
- [ ] `m2-auth`: `Auth` model, inheritance collection > folder > request, Basic and Bearer applied when sending, auth tab showing where the effective auth comes from.
- [ ] `m2-history`: local history in `.damnhttp/`, list and reopen, never committed.

## M3: git engine, simple Sync UI, request-level diff

- [ ] `m3-plan`: detailed breakdown, sync state machine, conflict strategy. Ask the maintainer about the UX flows.
- [ ] `m3-thin-slice`: open a workspace from a Git URL, edit a request, Save version, Sync, see the change on a second clone. **Then stop and tell the maintainer** (test with a non-developer teammate).
- [ ] Rest of M3: `GitEngine` trait complete, git detection screen, connection check, "Your changes" request-level diff, "Updates from your team", Advanced toggle (branches, log, raw status), file watcher.

## M4: conflicts, background fetch, error mapping, leak guard

- [ ] Field-level conflict resolution UI, background fetch with badge, friendly git error mapping, secret-leak guard on "Save version" (UI on top of the M2 rules module), `docs/GIT_AUTH.md`.

## M5: importers

- [ ] Import framework (preview + report), Postman v2.1 collections and environments, Insomnia v4, Bruno `.bru`, OpenAPI 3.x / Swagger 2.0 (file or URL), messy fixtures, path variable conversion.

## M6: polish

- [ ] UX review, docs and user guides, i18n and theme cleanup, `docs/FORMAT.md` as a finished spec (including the URL-literal limitation, D-026, also in the README).

## M7: release pipeline

- [ ] semantic-release with `main` / `beta` channels, tag-driven version stamping + CI check, single release workflow (dry-run version, build matrix, publish with `SHA256SUMS`), build-only matrix on PRs or nightly, `docs/RELEASING.md`, end-to-end verification on `beta`.
