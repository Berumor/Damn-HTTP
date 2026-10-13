# Decisions

ADR-style log. Newest at the bottom. Status is `accepted` (from the brief or approved by the maintainer), `proposed` (made by an agent, awaiting review in the PR that introduced it) or `superseded by D-nnn`. D-001 to D-018 were approved by the merge of PR #1, with the changes the maintainer made in that PR's comment (D-019 to D-021). D-022 and D-024 to D-029 come from the maintainer's answers in PR #2. D-034 to D-037 are agent proposals awaiting review in the `m1-file-format` PR. Never delete an entry; supersede it with a new one.

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

## D-022: Requests get a stable `id` (2026-10-09, accepted; supersedes D-006)

- Context: D-021 keys local values "by request". With identity = file path (D-006), a teammate renaming or moving a request would orphan everyone else's local values on the next sync.
- Decision: requests carry an `id` (`req_` + 8 random base32 characters) generated once at creation and never changed. Environments keep their `id`. Folders and collections have none; their identity is the path. "Duplicate" generates a new id. If two files share an id (hand copy), the app warns and assigns a new id to the one that is new in git on its next save.
- Alternatives: key local values by path and re-key them from git rename detection after each sync (fragile, and fails for a rename plus a large edit).
- Consequences: one extra line per request file. History and open tabs can also key by id.
- Amended by the maintainer in PR #2 (this replaces the id format and duplicate rule above):
  - The id is a UUID v4 (lowercase, hyphenated), generated once at creation, never changed on save, and written as the **first line** of the request file. Environment ids use the same format.
  - Duplicate ids are detected on load (a file copied by hand). The copy gets a new id: the copy is the file that is untracked or newer in git; if that cannot be told, the one whose path sorts later.
  - The Duplicate action creates a new id and copies the local values.
  - A "Clear unused local data" action removes local values whose request id no longer exists.

## D-023: Merge commits everywhere, no squash (2026-10-09, accepted; supersedes the squash part of B-12)

- Context: the brief says to squash `feature/*` and `fix/*` into `develop`. The maintainer said on 2026-10-09 that they prefer to keep the history, and merged PR #1 with a merge commit.
- Decision: every PR is merged with a merge commit, including `feature/*` and `fix/*` → `develop`. Squash and rebase merging are not used.
- Alternatives: squash into `develop` (the brief's original rule).
- Consequences:
  - Every commit on a feature branch reaches `develop`, `beta` and `main`, and semantic-release reads each of them. So **every commit message must be a valid Conventional Commit**, not only the PR title. `m0-ci` adds a check on all commits of a PR. The PR title check stays.
  - Each `feat:` and `fix:` commit becomes a line in the release notes. Follow-up corrections inside a branch use the type that describes them for a user (`refactor:`, `test:`, `chore:`, `docs:`), or are folded into the commit they correct **before the branch is pushed**. Pushed history is never rewritten.
  - The brief's rule "the app builds and tests pass at each PR" still applies to the PR head, not to every intermediate commit.
  - Repository setting: allow merge commits only.

## D-024: A value is a reference only if it is exactly one `{{name}}` (2026-10-09, accepted)

- Context: D-021 commits query and path variable values only when they are references.
- Decision: after trimming, the whole value must be exactly one `{{name}}`; spaces inside the braces are allowed (`{{ name }}`). Anything else, including `{{a}}-1` and `{{a}}{{b}}`, is a literal and stays local as a whole. The UI shows a short explanation when a value is kept local for this reason.
- Consequences: one simple test decides where a value is stored. Files are written in the canonical form `{{name}}`.

## D-025: D-021 covers query params and path variables only in v1 (2026-10-09, accepted)

- Decision: header values, bodies and form fields are committed as written. The secret-leak guard scans all of them.
- Consequences: a per-field "keep local" option for headers and body fields is a possible later feature (see `NOTES.md`).

## D-026: Literals in the URL path or host are accepted, with a soft hint (2026-10-09, accepted)

- Context: in `/it/stations/1234` the app cannot tell data from fixed path text.
- Decision: accept the limitation and document it in `docs/FORMAT.md` and the README. Add a hint, never blocking, in the same rules module as the leak guard: for a path segment that is purely numeric, a UUID, or a 24-character hex string, offer "Turn this into a path variable?", which rewrites the segment to `:id` and moves the value to local values. Tests hold it to a low false-positive bar.
- Consequences: short numeric segments such as `/v1` or `/2024` contain letters or are plausible fixed text; the rule set must not fire on API version segments.

## D-027: "Share with team" and "Make local again" (2026-10-09, accepted)

- Decision: "Share with team" on a local query or path variable value moves the literal into a collection variable (committed, not secret) and replaces the value with `{{name}}`. The name is suggested from the param name; if that name exists with a different value, the user is asked for another. The leak guard runs on the value first, and a confirmation says the value will be committed and visible to the team. "Make local again" reverses it: the param gets the literal back as a local value. Collection scope only in v1.
- Consequences: "Make local again" leaves the collection variable in place when other requests still reference it.

## D-028: Sparse `path_params`, one shared URL parser (2026-10-09, accepted)

- Decision:
  - `path_params` in a request file lists a variable only when it has a `{{name}}` value or a description. Names are always derived from `url`.
  - One parser, in `core`, derives them, and both the file loader and the UI use it. The UI calls it through a Tauri command (`url_analyze`) and has no parser of its own.
  - The parser ignores the scheme, ports such as `localhost:8080`, and anything inside `{{ }}`. Names match `[A-Za-z_][A-Za-z0-9_]*`. This extends D-013.
  - An entry whose name no longer appears in the URL is flagged on load and dropped on the next save.
  - When a name changes in the URL, its local value and sparse entry move with it. A change counts as a rename when exactly one name disappears and one appears in the same edit.
- Alternatives: a second parser in TypeScript for instant highlighting (two implementations that can disagree).
- Consequences: URL highlighting in the editor is asynchronous (debounced IPC call).

## D-029: The detection rules module is built in M2, its commit-time UI in M4 (2026-10-09, accepted)

- Context: "Share with team" (M2) must run the leak guard on the value, but the brief places the leak guard in M4.
- Decision: the rules module in `core` (credential patterns, entropy, the D-026 path hint), with its tests and documentation, is task `m2-detection-rules`. The "Save version" warning flow that uses it stays in M4.
- Consequences: one rules module from the start; M4 only adds UI on top of it.

## D-030: Repo hygiene choices (2026-10-09, accepted)

- Decision: Code of Conduct is the Contributor Covenant 2.1. Security reports go through GitHub private vulnerability reporting. Blank issues are disabled; bug and feature requests use issue forms. `NOTICE` names "Berumor and the Damn HTTP contributors" as copyright holder.
- Alternatives: a security email address (none exists yet, and none is to be registered).
- Consequences: the maintainer has to enable private vulnerability reporting and decide on a public contact (see `NOTES.md`).

## D-031: Cargo workspace settings (2026-10-09, accepted)

- Decision:
  - Toolchain pinned to an exact stable version (`1.99.0`) in `rust-toolchain.toml`; `rust-version = "1.99"`; edition 2024; resolver 3.
  - Lint policy lives in `[workspace.lints]`: `unsafe_code` forbidden; clippy `unwrap_used`, `expect_used`, `panic`, `todo`, `dbg_macro`, `print_stdout`, `print_stderr` as warnings, which CI turns into errors. `clippy.toml` allows them in tests.
  - `deny.toml` starts with a short allow list of permissive licenses (Apache-2.0, MIT, BSD-2/3-Clause, ISC, Unicode-3.0, Zlib, Apache-2.0 WITH LLVM-exception). Yanked crates, wildcard versions, unknown registries and git sources are denied. Other licenses are added only when a dependency needs them. Tauri is known to pull in MPL-2.0 crates; that addition will be reviewed in `m1-tauri-skeleton`.
  - Workspace crates are `publish = false`.
- Alternatives: a floating `stable` channel (CI and local builds can then disagree, and a new clippy release can break an unrelated PR).
- Consequences: toolchain updates are explicit `chore:` PRs.

## D-032: CI layout (2026-10-09, accepted)

- Decision: one workflow, `ci.yml`, with four jobs: Rust (fmt, clippy, tests), cargo-deny, PR title, Commits. The Commits job is a shell script in the repo that checks Conventional Commits and DCO sign-off on every non-merge commit of the PR (D-015, D-023). Allowed types: `feat`, `fix`, `docs`, `test`, `chore`, `refactor`, `ci`, `build`, `perf`, `revert`. Rust jobs run on `ubuntu-24.04` only; other platforms are covered by the packaging check in M1. Actions are pinned by SHA. The workflow token is read-only.
- Alternatives: commitlint (needs Node tooling in the repo before the frontend exists); running Rust tests on all three OSes on every PR (slow; revisit when platform-specific code such as git process handling arrives in M3).
- Consequences: the sign-off must carry the commit author's email, so commits authored by bots (e.g. Dependabot) would fail until an exemption is added.

## D-033: The committed / local split is enforced by a type (2026-10-09, accepted)

- Context: D-021 says a literal query or path variable value must never reach a committed file by accident.
- Decision: the editor works on `Request`, which holds every value. `Request::split()` returns a `CommittedRequest` and the `RequestLocalValues`. `CommittedRequest` has no other constructor, and the file writer (`m1-file-format`) will accept only that type. The file loader also goes through `split()`, so a literal in a hand-edited file is moved to local values on load. `CommittedRequest::merge()` puts local values back; where the committed file holds a reference, the reference wins over a stale local value.
- Details: repeated query names are matched by occurrence index among parameters of the same name. An empty value is stored nowhere. References are rewritten to the canonical `{{name}}`.
- Alternatives: one `Request` type and a check in the serializer (a forgotten check leaks a value; a type cannot be forgotten).
- Consequences: the model has no `serde` derives yet (`m1-file-format` did not need them: the format is mapped by hand, D-035). They may be added in `m1-tauri-skeleton`, and `CommittedRequest` must not gain a public constructor or a `Deserialize` impl that bypasses `split()`. The same pattern will be used for secret environment values in `m2-secrets`.

## D-034: `saphyr-parser` reads YAML (2026-10-12, accepted)

- Context: D-008 leaves the choice of the YAML parser to `m1-file-format`. Output is our own emitter; the parser only has to read.
- Decision: `saphyr-parser` 0.2.1, pinned exactly (`=0.2.1`). It is an event parser (MIT OR Apache-2.0, YAML 1.2, passes the yaml-test-suite, dependencies `arraydeque` and `thiserror` only). We build our own small tree from its events, keeping scalar styles, anchors, tags and positions, which a loaded `Yaml` value would hide.
- Alternatives: `yaml-rust2` (same lineage; its `encoding` default feature pulls `encoding_rs`, and `deny.toml` checks all features); `saphyr` (adds a loader we do not need); serde-based crates such as `serde-saphyr` (typing by serde instead of by our schema; `serde_yml` is deprecated).
- Consequences: a 0.x crate, so upgrades are deliberate: bump the pin in its own PR and run the format tests. Its columns are 0-based although documented as 1-based; `format::tree` adds 1.

## D-035: How files are read (2026-10-12, accepted)

- Decision:
  - Scalars are typed by the key, not by YAML resolution: string keys take the text of any scalar (`value: 8080` is `"8080"`, `name: no` is `"no"`); booleans are only plain `true` / `false`; numbers only plain digits. An empty value, `~` or `null` counts as absent.
  - Any YAML 1.2 style is accepted on read (flow collections, single quotes, folded blocks, CRLF, BOM, any key order). Anchors, aliases, tags, several documents, complex keys and duplicate keys are errors. Nesting deeper than 32 is an error.
  - Unknown keys are errors (D-009); `version` is checked before anything else, so a newer file reports `NewerVersion` and not an unknown key.
  - Reading checks structure only. Semantic problems (empty names, duplicate variables, a file in a URL-encoded body) are left to `validate()`, so the file still opens and the UI can show the problem.
  - Errors carry line and column and name keys, never values.
- Alternatives: YAML's own typing (the "Norway problem": `no` becomes `false`); refusing non-canonical styles (hostile to hand edits).
- Consequences: a hand-written file may differ from the canonical form; the next save rewrites it canonically.

## D-036: Writer details (2026-10-12, accepted)

- Decision:
  - A string is written plain only when no YAML 1.1 or 1.2 reader could read it as anything else; the rule is deliberately conservative (anything starting with a digit, `+` or `.` is quoted, except a UUID). Otherwise double-quoted, with control and invisible characters escaped. Multi-line text uses `|`, `|-` or `|+` (with an indentation indicator when the first line starts with a space); text with `\r` or characters needing escapes is double-quoted instead.
  - Empty lists and maps are never written. `method` and `url` are always written on requests (`url: ""` when empty).
  - Form bodies use one `fields` list for both kinds; a field has `value` or `file`.
  - Collection `auth` is left out when `none` (its default); folder and request `auth` when `inherit`.
  - `.damnhttp/values.yaml` also has `kind: values`, so every file the app writes identifies itself.
- Consequences: these choices are now the v1 format (expensive to change once teams have files).

## D-037: JSON Schemas (2026-10-12, accepted)

- Decision: one schema per kind in `schemas/v1/`, a `common.schema.json` for shared definitions, and `file.schema.json` that selects the schema by `kind`, so an editor needs one glob. Draft 2020-12. `$id`s point at `raw.githubusercontent.com/Berumor/Damn-HTTP/main/schemas/v1/`. The schemas flag literal query / path values and secret values, which the app would move out of the file. A test validates the fixtures, the writer's output and known-bad files with `boon` (dev-dependency only).
- Alternatives: one schema per kind only (editors would need per-file globs that cannot tell `folder.yaml` from a request); `jsonschema` crate for the test (MIT only, and with all features it pulls Python and Ruby bindings into the `cargo-deny` graph).
- Consequences: the schemas expect YAML strings for string keys, so an editor flags `value: 8080` although the app accepts it. The `$id` URLs resolve only once the schemas reach `main`.

## D-038: Workspace store behaviour (2026-10-12, accepted)

- Context: `m1-workspace-store` had to settle details the plan left open.
- Decision:
  - **Order keys** follow the common fractional-indexing layout (integer part whose first character gives its length, then a fraction): `a0`, `a1`, ..., `a0V`. A sibling group is rewritten with fresh keys only when an item cannot be placed: equal neighbours, a hand-written key outside the layout, or a new key longer than 12 characters.
  - **Slugs** are ASCII only, at most 60 characters, with a small table for accented Latin letters; names with no usable character become `untitled`. Collisions are checked without case and across files and directories. Windows-reserved names and `collection` / `folder` get `-item`.
  - **Paths** given to the store must stay under `collections/`. Symbolic links and dot-files are skipped, never followed.
  - **Delete keeps local values.** Restoring the files with git brings the values back; "Clear unused local data" removes them. That action refuses to run while any file is unreadable, because an unreadable request's id is unknown.
  - **Duplicate ids:** the file whose path sorts first keeps the id; copies get a new id and a copy of the local values. The git-aware rule from D-022 (the untracked or newer file is the copy) needs the git engine and is added in M3.
  - **Duplicate** copies YAML items only; other files inside a folder (for example attachments for multipart bodies) are not copied.
  - **Rename is its own operation.** Saving a request never renames its file. The id of an existing file cannot be changed by a save.
  - **Opening a workspace adds `/.damnhttp/` to `.gitignore`** if no equivalent line is in effect.
  - **Writes are atomic** and skipped when the content is unchanged, so saving without changes does not touch the file.
- Alternatives: Unicode slugs (file names then differ between macOS and Linux for the same text); deleting local values together with the request (a `git restore` would bring back the request without them).
- Consequences: `docs/FORMAT.md` describes the slug and order-key rules. The store is synchronous; the `app` crate must call it off the UI thread.

## D-039: URL template parsing details (2026-10-12, accepted, except "Name end", superseded by D-040)

- Context: D-013 and D-028 define path variables; `m1-path-variables` had to settle the edge cases.
- Decision:
  - **Authority:** skipped from `scheme://` or a leading `//` to the next `/`; with no scheme, everything before the first `/` is the authority. This is what makes `localhost:8080/x`, `{{baseUrl}}/x` and `user:secret@host` safe.
  - **Name end:** a path variable's name ends at the first character that cannot be part of it, so `/files/:name.pdf` contains the variable `name`. (Postman treats the whole segment as the name.)
  - **Query parameters are kept as typed.** Splitting a pasted URL does not percent-decode, and the URL bar text is rebuilt without encoding, so URL bar → table → URL bar is lossless. How parameters are encoded when sent is decided in `m1-http-engine`.
  - **Path variable values are always percent-encoded** when substituted (everything but unreserved characters, including `/` and `%`). `{{references}}` inside a value are left for the variable resolver.
  - **A path variable without a value stays in the URL** as `:name` and is reported, so the UI can warn before sending.
  - **Rename detection:** editing the URL so that exactly one variable name disappears and one appears moves the value and description to the new name, unless an unused entry already has that name (then that entry's own value comes back).
  - **Enforcement:** `Request::split` moves any query string out of `url`, so the D-021 guarantee does not depend on callers splitting pasted URLs. Saving through the store drops path variable entries that are no longer in the URL.
  - **URL bar and disabled rows:** editing the URL bar rewrites the enabled query rows in order and leaves disabled rows where they are.
- Alternatives: decoding query values on paste (`+` and `%2B` then become ambiguous on the way back); a TypeScript copy of the parser for instant highlighting (rejected in D-028).
- Consequences: `m2-variable-engine` must percent-encode the value of a reference that sits inside a path variable value. Offsets from `analyze` are bytes; the `url_analyze` command converts them to UTF-16 for the webview.

## D-040: A path variable is a whole segment, as in Postman (2026-10-13, accepted)

- Context: D-039 proposed that a name ends at the first character that cannot be part of it (`/files/:name.pdf` → `name`). The maintainer answered in PR #10: "Treat like Postman". The other three points of D-039 (query parameters kept as typed, path values always percent-encoded, a variable without a value left as `:name`) were confirmed there.
- Decision: a path variable is a whole path segment of the form `:name`; the name runs to the next `/` or the end of the path. `/files/:name.pdf` has the variable `name.pdf`. To make that possible the name grammar is widened from `[A-Za-z_][A-Za-z0-9_]*` (D-013, D-028) to `[A-Za-z_][A-Za-z0-9_.-]*`. A segment that starts with `:` but is not such a name, including one that contains a `{{reference}}`, is ordinary text.
- Alternatives: keep the strict grammar and treat `/:name.pdf` as plain text (the user's variable would silently not exist); allow any character in the name as Postman does (names then need escaping in files and in the UI).
- Consequences: importers can keep Postman path variable names with dots and dashes unchanged; other names (spaces, non-ASCII) have to be converted and reported. `schemas/v1/request.schema.json` and `docs/FORMAT.md` use the new grammar. No format version bump: no release has shipped the old rule.

## D-041: HTTP engine choices (2026-10-13, accepted)

- Context: `m1-http-engine`. The brief fixes `reqwest` + `rustls`, sent from Rust.
- Decision:
  - **TLS:** reqwest's `rustls` feature: the `aws-lc-rs` crypto provider and the platform certificate verifier, so certificates trusted by the operating system (including a company CA installed there) are trusted. Custom CAs are added to the system's, not a replacement. No native TLS, no OpenSSL, no `ring` in the build.
  - **Query encoding (left open by D-039):** the URL is sent as typed. Only characters that cannot appear in a URL are percent-encoded (a space becomes `%20`, non-ASCII becomes UTF-8 `%XX`); existing `%XX` sequences and `+` are not touched.
  - **No scheme means `http://`**, so `localhost:8080/x` works as typed. Only `http` and `https` are sent.
  - **A leftover `{{variable}}` is an error** (`UnresolvedVariables`, naming the variables) instead of being sent percent-encoded.
  - **Content-Type:** implied by the body (`application/json` etc. from the caller, `application/x-www-form-urlencoded`, multipart with a generated boundary). A `Content-Type` header typed by the user replaces it, except for multipart, where the typed header is ignored because it could not carry the right boundary.
  - **Defaults:** `User-Agent: DamnHTTP/<version>` unless the user sets one; no `Referer` on redirects; 30 s timeout for the whole exchange; up to 10 redirects. `max_redirects: 0` means "do not follow".
  - **Response body on disk**, streamed, decompressed (gzip, deflate, brotli). `body_size` is the decompressed size.
  - **Timing** has `time_to_headers`, `download` and `total`. DNS, connect and TLS times are not separated: reqwest does not expose them, and getting them means replacing its connector.
  - **One client per request.** Connections are not reused between sends, so every send includes connection setup.
  - **Errors** carry a `kind` for the UI and a `detail` for diagnostics. The URL is stripped from transport errors and header values never appear, since both can hold secrets.
  - `deny.toml` now allows `CDLA-Permissive-2.0`, the permissive data license of the Mozilla root certificate list (`webpki-root-certs`).
- Alternatives: `ring` as crypto provider (reqwest's default is now `aws-lc-rs`; using `ring` means installing a process-wide provider by hand); a shared client (wrong settings would leak between requests); zstd decompression (needs a C library; add it if asked for).
- Consequences: `aws-lc-sys` compiles C code, so Windows and macOS builds need a C compiler; `m1-packaging-check` will show whether CI needs anything more. `status_text` is the standard phrase for the code, not what the server sent.

## D-042: App shell and frontend scaffolding (2026-10-13, accepted; its ESLint and Prettier settings are replaced by D-043)

- Context: `m1-tauri-skeleton`. The brief asks for restrictive capabilities, typed commands, the identifier in one place, strict TypeScript, i18n and themes.
- Decision:
  - **Commands are denied by default.** `build.rs` declares the app's commands in an app manifest, so each needs an explicit `allow-<command>` permission in `capabilities/default.json`. The capability grants nothing from Tauri's core or from plugins. Two tests keep the bindings, `build.rs` and the capability file in agreement.
  - **CSP:** only the app's own files and the IPC endpoint; inline styles allowed (CodeMirror needs them later); inline scripts only in development for Vite.
  - **Bindings freshness is a Rust test**, so the existing CI job catches a stale `ui/src/bindings.ts`.
  - **The identifier `com.damnhttp.app` is only in `crates/app/tauri.conf.json`.** The version is not there at all; Tauri reads it from Cargo.
  - **TypeScript 6.0, not 7.** `typescript-eslint` does not support TypeScript 7 yet. Revisit when it does.
  - **ESLint** uses typescript-eslint's strict type-checked rules and forbids importing `invoke` directly. **Prettier** covers the frontend only.
  - **Theme** follows the operating system by default, with a manual choice stored in the webview's local storage until the settings commands exist.
  - **Licenses:** MPL-2.0 is allowed for five named crates that Tauri depends on (`cssparser`, `cssparser-macros`, `dtoa-short`, `selectors`, `option-ext`), not globally; `NOTICE` lists them. Two "unmaintained" advisories for build-time macro crates (`paste` via specta, `proc-macro-error` via gtk-rs) are ignored with a reason in `deny.toml`.
  - **Node 26** pinned in `.node-version`; pnpm pinned by `packageManager`.
  - **`@testing-library/user-event` is not used:** pnpm's release-age guard flagged its current version as too new and wanted an exception. Tests use `fireEvent`; add `user-event` once a version passes the guard.
  - **The icon is a placeholder** drawn by a script (a `>_` prompt on a dark tile).
- Alternatives: Tauri's default behaviour, where every registered command is callable from every window; a CI step that regenerates bindings and diffs them (needs the frontend toolchain in the Rust job).
- Consequences: adding a command touches four places (see `CONVENTIONS.md`); forgetting one fails a test, not a user.

## D-043: Default settings for Prettier, ESLint and the test runner (2026-10-13, accepted)

- Context: the maintainer's review of PR #13: "Use the default value for prettier, eslint and for test". The other points of D-042 were approved there (TypeScript 6.0, theme in local storage for now, MPL-2.0 per crate, no `user-event` for now, placeholder icon until the designer delivers one).
- Decision:
  - **Prettier:** no configuration file; all defaults (80 columns, double quotes, semicolons). `.prettierignore` still limits it to the frontend.
  - **ESLint:** the `recommended` sets of ESLint, typescript-eslint and the React hooks plugin, unchanged. No type-checked or `strict` sets.
  - **Vitest:** defaults, plus the two settings without which React component tests cannot run: the `jsdom` environment and one setup file (matchers, cleanup, i18n).
  - **Kept, because it is architecture and not style:** the rule that forbids importing `invoke` directly, so the frontend can only call Rust through the generated typed commands (brief requirement). `tsconfig.json` stays strict, also a brief requirement.
- Alternatives: the stricter type-checked ESLint sets from D-042 (more findings, more configuration to understand).
- Consequences: do not add rule overrides or formatter options without asking the maintainer.

## D-044: Packaging check workflow (2026-10-13, proposed)

- Context: `m1-packaging-check`; the brief wants a build-only packaging check on Linux, macOS and Windows in M1, and the same matrix build-only on pull requests later (M7).
- Decision: one workflow, `packaging.yml`, with a four-target matrix (Linux AppImage on `ubuntu-22.04`, macOS arm64 and x86_64 `.dmg`, Windows NSIS). It runs `pnpm tauri build` directly, uploads the installers as artifacts for 3 days and publishes nothing. Triggers: pull requests to `develop` that touch app code or build configuration, and manual runs. Both macOS targets build on the Apple Silicon runner; Intel is cross-compiled.
- Alternatives: `tauri-apps/tauri-action` now (it is planned for the release workflow in M7, where it also creates the release; for a plain build the CLI is the same thing with one fewer third-party action); a nightly schedule (wasteful while the repository is quiet; the path filter already covers every change that can break packaging).
- Consequences: not a required check, because it does not run on documentation-only pull requests and takes several minutes. M7 turns this matrix into the release workflow.
