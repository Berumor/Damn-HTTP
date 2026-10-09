# Decisions

ADR-style log. Newest at the bottom. Status is `accepted` (from the brief or approved by the maintainer), `proposed` (made by an agent, awaiting review in the PR that introduced it) or `superseded by D-nnn`. D-001 to D-018 were approved by the merge of PR #1, with the changes the maintainer made in that PR's comment (D-019 to D-021). Never delete an entry; supersede it with a new one.

## Decisions made in the brief (2026-10-09, accepted)

| ID | Decision | Alternatives rejected | Consequences |
|---|---|---|---|
| B-01 | License Apache-2.0, DCO sign-off, no CLA | Other licenses, CLA | Every dependency must be Apache-2.0 compatible (`cargo-deny`); every commit needs `Signed-off-by`. |
| B-02 | v1 is REST only | GraphQL, gRPC, WebSocket | Model and UI assume one request, one response. |
| B-03 | Tauri 2, Rust backend, React + Vite + TS strict, Zustand | Electron, other UI stacks | Linux webview is WebKitGTK: keep the UI light, test there early. |
| B-04 | CodeMirror 6 for editors | Monaco | Lighter in WebKitGTK; language features built from CM6 extensions. |
| B-05 | All HTTP is sent from Rust (`reqwest` + `rustls`) | `fetch` in the webview | No CORS limits, full control of TLS, proxy, timing; the webview needs no network permission. |
| B-06 | Local git via `git2` or `gix`; all network git via the system `git` executable; both behind `GitEngine` | Network through libgit2, bundled git | The user's credentials, SSH agent, helpers and signing work unchanged. No system git = local-only mode. |
| B-07 | The app never stores or asks for git credentials | OAuth, token fields, keychain entries for git | Auth problems are solved outside the app; we ship `docs/GIT_AUTH.md` and friendly errors. |
| B-08 | One YAML file per request, deterministic output, versioned schema, no volatile fields | Single collection file, JSON, custom DSL | Clean diffs, rare conflicts; we own serialization details. |
| B-09 | Path variables use `:id`; `{{var}}` is a separate mechanism | `{id}` or `{{id}}` for paths | Parser must tell `:id` from ports; importers convert other syntaxes. |
| B-10 | Secret values only in a git-ignored file or the OS keychain | Encrypted secrets in the repo | Teammates re-enter secrets; a leak guard warns before commit and never blocks hard. |
| B-11 | Git vocabulary hidden by default ("Sync", "Save version", ...), Advanced toggle | Git-first UI | Simple mode stays on the default branch and never force-pushes. |
| B-12 | Git flow: `main` / `beta` / `develop` + `feature/*`, `fix/*`; squash into `develop`, merge commits for promotions. **The squash part is superseded by D-023.** | Trunk-based | Promotion history must stay intact for release tooling. |
| B-13 | semantic-release with `main` (stable) and `beta` (prerelease) channels | release-please | Versions come from Conventional Commits. |
| B-14 | The git tag is the only source of the version; repo files stay at `0.0.0`, CI stamps at build time | `@semantic-release/git` bump commits | No bot commits on protected branches; a CI check must prove stamping works. |
| B-15 | v1 binaries unsigned; Linux ships AppImage only | Signing, Flatpak / deb / rpm | README must explain Gatekeeper, SmartScreen, AppImage requirements. |
| B-16 | No telemetry, no cloud accounts, no scripting, no CLI in v1 | | `core` and `http` stay UI-free so a CLI can be added later. |

## D-001: Package names `damnhttp-*`, directories `crates/<name>` (2026-10-09, accepted)

- Context: the brief names the crates `core`, `http`, `import`, `git`, `app`. A package named `core` shadows Rust's built-in `core`.
- Decision: directories keep the brief's names; packages are `damnhttp-core`, `damnhttp-http`, `damnhttp-import`, `damnhttp-git`, `damnhttp-app`. Frontend sources live in `ui/`, with `package.json` at the repo root so the Tauri CLI runs from the root.
- Alternatives: `src-tauri/` layout (hides the workspace structure); frontend in its own package directory (Tauri CLI then needs extra path config).
- Consequences: `tauri.conf.json` lives in `crates/app/`.

## D-002: pnpm as the Node package manager (2026-10-09, accepted)

- Decision: pnpm with a committed lockfile, version pinned through `packageManager`.
- Alternatives: npm (slower, looser), bun (less proven with Tauri CI on Windows).
- Consequences: contributors need pnpm (via corepack).

## D-003: tauri-specta for TypeScript bindings (2026-10-09, accepted)

- Context: the frontend must talk to Rust only through typed commands.
- Decision: `specta` + `tauri-specta` generate `ui/bindings.ts` (types and command wrappers). Versions pinned exactly, since tauri-specta 2 has shipped as release candidates. CI fails if the generated file is stale.
- Alternatives: `ts-rs` (types only; command names and argument shapes stay stringly typed).
- Consequences: `core` gets an optional `specta` feature. If tauri-specta blocks an upgrade, fall back to ts-rs plus hand-written wrappers.

## D-004: `git2` with vendored libgit2, no network features (2026-10-09, accepted)

- Decision: `git2` with default features off and `vendored-libgit2` on. No `https` / `ssh` features, so no OpenSSL or libssh2 in the build.
- Alternatives: `gix` (pure Rust, MIT/Apache; its status, index-conflict and worktree-write APIs are less complete for what conflict resolution needs).
- Consequences: libgit2 is GPL-2.0 **with a linking exception** that allows linking into a binary under any license. Compatible with shipping Apache-2.0 binaries, but it must be stated in `NOTICE`, and it is the one dependency `cargo-deny` cannot judge from crate metadata. The `GitEngine` trait keeps a later move to `gix` cheap. Confirmed by the maintainer in PR #1: keep `git2`, document the exception in `deny.toml` and `NOTICE`.

## D-005: Workspace = repository; several collections; environments at workspace level (2026-10-09, accepted)

- Context: the brief says "workspace > folders > requests" and also "collection-level variables" and a "collection-level default" auth.
- Decision: a workspace is a folder (normally a git repo root) with `damnhttp.yaml`, `collections/<slug>/` and `environments/<slug>.yaml`. A workspace holds one or more collections. Environments are shared by all collections in the workspace.
- Alternatives: one collection per repo (simpler, but teams with several APIs need several repos); environments per collection, as in Bruno (dev/staging/prod get duplicated per collection).
- Consequences: expensive to change after M1. Maintainer in PR #1: "a single repository is fine. No split." Read as approval of this layout.

## D-006: Identity is the file path; only environments get an `id` (2026-10-09, superseded by D-022)

- Decision: requests, folders and collections have no ID; the file name is a slug of `name`. Environments carry an `id` generated once at creation (`env_` + 8 random base32 characters).
- Why: local secrets are keyed by environment + variable name and must survive a teammate renaming the environment. Nothing local points at requests in a way that must survive renames (history stores snapshots).
- Alternatives: IDs everywhere (noise in every file, duplicate-ID problems when files are copied by hand); no IDs at all (secrets orphaned on environment rename).
- Consequences: request rename tracking in diffs relies on git rename detection.

## D-007: Sibling order stored as a fractional `order` key per item (2026-10-09, accepted)

- Decision: every collection, folder and request has an `order` string (fractional index). Siblings sort by `order`, then file name.
- Alternatives: an ordered list of children in the parent file (two people adding to the same folder conflict on the list; rename touches two files); integer `seq` as in Bruno (reordering rewrites many files).
- Consequences: a move changes one field in one file and two concurrent additions never conflict. The key is not meaningful to a human reader. Maintainer in PR #1: break ties by file name, renormalize the keys of a sibling group when they grow too long, and document the rule in `FORMAT.md`.

## D-008: Own deterministic YAML emitter over a restricted subset (2026-10-09, accepted)

- Context: byte-identical output is a hard requirement, and `serde_yaml` is unmaintained.
- Decision: `core` writes files with its own emitter (maps, lists, plain / double-quoted / `|` block scalars; fixed key order per kind; explicit quoting rules). Reading uses a maintained YAML parser, chosen in `m1-file-format` and license-checked by `cargo-deny`, then maps into the typed model with strict validation.
- Alternatives: rely on a serde YAML library for output (quoting and layout can change between library versions and silently rewrite every file).
- Consequences: more code and tests in `core`; output is stable regardless of dependency upgrades. No anchors, aliases, tags or flow collections in files we write.

## D-009: Per-file `version`; newer files open read-only (2026-10-09, accepted)

- Decision: every file carries `version: 1`. Older versions are migrated when written. A file with a version newer than the app supports is shown read-only with a prompt to update the app. Unknown keys in a supported version are a validation error.
- Alternatives: preserve unknown keys on round-trip (conflicts with deterministic key ordering and hides typos).
- Consequences: a team should update the app together when the format version changes; FORMAT.md must say so.

## D-010: Secrets in the OS keychain by default, git-ignored file as fallback; environment variables only (2026-10-09, superseded by D-019)

- Decision: `SecretStore` trait with two backends. Default is the OS keychain (`keyring`); if it is unavailable (e.g. Linux without a Secret Service), fall back to `.damnhttp/secrets.yaml` and say so in the UI. Only environment variables can be secret in v1; collection variables cannot.
- Alternatives: file only (plain text on disk for everyone); keychain only (breaks on minimal Linux desktops).
- Consequences: the leak guard's "move to a secret variable" needs an environment, and creates one if none exists.

## D-011: Proxy and custom CA are local settings, not committed (2026-10-09, superseded by D-020)

- Context: the brief asks for redirects, timeouts, proxy, custom CA and skip-TLS "per request".
- Decision: `http` accepts all of them per request. Request files store `follow_redirects`, `max_redirects`, `timeout_ms`, `verify_tls`. Proxy and custom CA paths are machine-specific, so they are stored in app settings with an optional per-workspace override in `.damnhttp/`.
- Alternatives: commit proxy / CA paths in request files (they break for every teammate with a different machine).
- Consequences: no per-request proxy override in the v1 UI.

## D-012: Response bodies stay in the backend and are read by handle (2026-10-09, accepted)

- Decision: `http_send` returns metadata and a `response_id`. The body is written to a temp file and read by the frontend in byte ranges as raw IPC payloads; JSON pretty-printing is done in Rust. Temp files are removed when the tab closes or the app exits.
- Alternatives: return the body inside the JSON command result (freezes the webview on multi-MB responses).
- Consequences: "save to file" and search over huge bodies can work without loading everything into the webview.

## D-013: Path variable grammar (2026-10-09, accepted)

- Decision: `:name` is a path variable only when `:` directly follows `/` in the path part of the URL template and `name` matches `[A-Za-z_][A-Za-z0-9_]*`. Path variables are substituted before `{{var}}` interpolation; values are percent-encoded.
- Alternatives: any `:word` anywhere in the URL (breaks ports and `a:b` segments).
- Consequences: a literal segment starting with `:` must be written percent-encoded (`%3A`).

## D-014: Variable precedence environment > collection (2026-10-09, accepted)

- Decision: when a name exists in both scopes, the active environment wins. Matches Postman, so imports behave the same.
- Consequences: the hover tooltip shows which scope a value came from.

## D-015: DCO and PR title checks run as plain workflows (2026-10-09, accepted)

- Decision: the DCO check is a workflow step that verifies `Signed-off-by` on every commit of the PR. The PR title check uses `amannn/action-semantic-pull-request`, pinned by commit SHA. All third-party actions are pinned by SHA.
- Alternatives: the DCO GitHub App (needs an install by the maintainer and an external service).
- Consequences: nothing to install outside the repo.

## D-016: Local state lives in `<workspace>/.damnhttp/` (2026-10-09, accepted)

- Decision: history, the secrets fallback file, active environment and UI state for a workspace are stored in `.damnhttp/` inside the workspace. The app adds `/.damnhttp/` to `.gitignore` on create, open and clone, and checks before every "Save version" that the path is still ignored and untracked.
- Alternatives: the OS app-data directory keyed by workspace path (state is lost when the folder moves).
- Consequences: follows the brief ("git-ignored paths"); the pre-commit check covers a hand-edited `.gitignore`.

## D-017: History is built in M2 (2026-10-09, accepted)

- Context: the brief lists History as a v1 feature but assigns it to no milestone.
- Decision: last task of M2 (`m2-history`), once environments exist, so entries can record the environment used.

## D-018: Frontend toolkit (2026-10-09, accepted)

- Decision: CSS variables + CSS modules for styling and themes, no component framework; `i18next` / `react-i18next`; `dnd-kit` for the tree; Vitest + Testing Library; Playwright for the smoke test.
- Alternatives: a full component kit (weight and WebKitGTK rendering risk).
- Consequences: a small set of in-house primitives (button, table, tabs, dialog) to build in `m1-tauri-skeleton` and `m1-workspace-ui`.

## D-019: Secrets live only in a git-ignored file (2026-10-09, accepted; supersedes D-010)

- Context: maintainer's answer in PR #1.
- Decision: secret values are stored in `.damnhttp/secrets.yaml`, always. No OS keychain backend in v1. Only environment variables can be marked secret; headers, params, bodies and auth fields cannot. The secret-leak guard still warns about credential-looking literals in any non-secret field and offers to move the value into a secret environment variable (creating an environment if none exists).
- Alternatives: keychain first with file fallback (D-010).
- Consequences: one code path on every OS; secrets are plain text on the user's disk. `SecretStore` stays a trait so a keychain backend can be added later. The `keyring` crate is not a dependency in v1.

## D-020: Proxy, custom CA and TLS verification are local settings (2026-10-09, accepted; supersedes D-011)

- Context: maintainer's answer in PR #1; the brief asks for skip-TLS "per request".
- Decision: proxy, custom CA and TLS verification are never committed. Proxy and CA are app settings with an optional per-workspace override in `.damnhttp/`. "Skip TLS verification" is a per-request local flag stored with the request's local values. Request files keep only `follow_redirects`, `max_redirects`, `timeout_ms`. No per-request proxy override in the v1 UI.
- Consequences: a teammate testing against a self-signed server sets the skip flag on their own machine.

## D-021: Literal values of query params and path variables are never committed (2026-10-09, accepted)

- Context: maintainer's rule in PR #1. Example: `https://some.service/:country/stations/?id=1234` is committed as URL `https://some.service/:country/stations/`, a query param named `id` and a path variable `country`; the values `1234` and `it` stay local.
- Decision:
  - Request files commit, for query params and path variables, only the name, the `enabled` flag, the order and the description. A `value` is committed only when it is a variable reference.
  - Literal values live in `.damnhttp/` (git-ignored), keyed by request `id` (D-022).
  - The committed `url` never contains a query string. A typed or pasted URL is split on save into committed URL + param names and local values.
  - A teammate who clones sees the same params and path variables with empty fields.
  - Importers drop literal values from committed files, keep them as local values where possible, and list them in the import report.
  - The "Your changes" diff shows only committed fields. The leak guard and the pre-commit check treat a literal query / path value in a committed file as a violation (it can only get there by hand-editing).
- Proposed details, awaiting the maintainer (see `NOTES.md`): a value counts as a reference only when it consists entirely of one `{{var}}` (anything else, including `{{a}}-1`, is local); the rule covers query params and path variables only, so header values, bodies and form fields are still committed.
- Alternatives: commit example values (what Postman, Bruno and Insomnia do).
- Consequences: a cloned request is not runnable until values are filled in. Values a team wants to share go through a collection or environment variable. A future `damnhttp run` CLI needs values supplied from variables.

## D-022: Requests get a stable `id` (2026-10-09, proposed; supersedes D-006)

- Context: D-021 keys local values "by request". With identity = file path (D-006), a teammate renaming or moving a request would orphan everyone else's local values on the next sync.
- Decision: requests carry an `id` (`req_` + 8 random base32 characters) generated once at creation and never changed. Environments keep their `id`. Folders and collections have none; their identity is the path. "Duplicate" generates a new id. If two files share an id (hand copy), the app warns and assigns a new id to the one that is new in git on its next save.
- Alternatives: key local values by path and re-key them from git rename detection after each sync (fragile, and fails for a rename plus a large edit).
- Consequences: one extra line per request file. History and open tabs can also key by id.

## D-023: Merge commits everywhere, no squash (2026-10-09, accepted; supersedes the squash part of B-12)

- Context: the brief says to squash `feature/*` and `fix/*` into `develop`. The maintainer said on 2026-10-09 that they prefer to keep the history, and merged PR #1 with a merge commit.
- Decision: every PR is merged with a merge commit, including `feature/*` and `fix/*` → `develop`. Squash and rebase merging are not used.
- Alternatives: squash into `develop` (the brief's original rule).
- Consequences:
  - Every commit on a feature branch reaches `develop`, `beta` and `main`, and semantic-release reads each of them. So **every commit message must be a valid Conventional Commit**, not only the PR title. `m0-ci` adds a check on all commits of a PR. The PR title check stays.
  - Each `feat:` and `fix:` commit becomes a line in the release notes. Follow-up corrections inside a branch use the type that describes them for a user (`refactor:`, `test:`, `chore:`, `docs:`), or are folded into the commit they correct **before the branch is pushed**. Pushed history is never rewritten.
  - The brief's rule "the app builds and tests pass at each PR" still applies to the PR head, not to every intermediate commit.
  - Repository setting: allow merge commits only.
