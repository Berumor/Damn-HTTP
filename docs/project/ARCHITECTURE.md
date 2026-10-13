# Architecture

State: the Cargo workspace exists. `core` has the data model (`m1-core-model`), the file format (`m1-file-format`), the workspace store (`m1-workspace-store`) and the URL parser (`m1-path-variables`); `http` has the request engine (`m1-http-engine`); `import` and `git` are empty. The `app` crate and the frontend arrive in `m1-tauri-skeleton`. Sections describing later tasks are the approved plan.

## Repository layout (planned)

```
Cargo.toml              Cargo workspace (version 0.0.0 placeholder, shared lints and deps)
rust-toolchain.toml     pinned stable toolchain
deny.toml               cargo-deny config (licenses compatible with Apache-2.0)
clippy.toml             lets tests use unwrap / expect / panic
rustfmt.toml
package.json            frontend + Tauri CLI scripts (pnpm)
vite.config.ts
crates/
  core/     damnhttp-core    models, file format, variables, path variables, auth inheritance,
                             secret-leak detection, workspace store. No Tauri, no network.
  http/     damnhttp-http    request execution (reqwest + rustls). No Tauri.
  import/   damnhttp-import  one module per source format, each returns (collections, report)
  git/      damnhttp-git     GitEngine trait + implementations
  app/      damnhttp-app     Tauri 2 shell: commands, state, events, capabilities, tauri.conf.json
ui/                     React + TypeScript (strict) sources
  bindings.ts           generated from Rust by tauri-specta; never edited by hand
docs/                   FORMAT.md, GIT_AUTH.md, RELEASING.md, user guides
docs/project/           project memory (this directory)
schemas/v1/             JSON Schemas for the file format, one per kind + file.schema.json
```

Directories are named `core`, `http`, ... as in the brief; packages are named `damnhttp-*` because a package called `core` would shadow Rust's built-in `core` crate (D-001).

## Dependency rules

```
app ──> core, http, git, import
http ──> core
import ──> core
git ──> core        (request-level diff and field-level merge use core models)
core ──> nothing in this workspace
```

- `core` and `http` never depend on Tauri or on anything UI-related. A future `damnhttp run` CLI is a new crate depending on `core` + `http`.
- `core` derives `specta::Type` behind a `specta` cargo feature, enabled only by `app`.

## Data flow

1. The frontend calls a typed command from `ui/bindings.ts`.
2. `app` validates the input, resolves workspace-relative paths (rejecting anything that escapes the workspace root), and calls into `core` / `http` / `git` / `import`.
3. The result comes back as a typed value or a typed `AppError { code, message, details }`. The frontend maps `code` to an i18n string.
4. Long-running work (send, clone, sync, import) runs on the async runtime, is cancellable by id, and reports through Tauri events.

The webview gets no filesystem, shell or HTTP permissions. File pickers and "save to file" are opened by the backend. The only capabilities granted are the app's own commands and core event listening.

Sending a request:

```
Request file ─┐
local values ─┤
collection /  ├─> core: resolve auth inheritance ─> substitute :path variables
folder chain  │         ─> interpolate {{vars}} (environment > collection) ─> ResolvedRequest
environment ──┤
local secrets ┘
ResolvedRequest ─> http: execute ─> ResponseMeta (+ body written to a temp file)
```

Response bodies are not serialized into the JSON result. `http_send` returns metadata plus a `response_id`; the frontend then reads the body in ranges as raw bytes. Pretty-printing JSON happens in Rust. This keeps multi-MB responses from freezing the webview (D-012).

## Data model (`core`)

Implemented in `crates/core/src/`:

| Module | Contents |
|---|---|
| `reference` | The `{{name}}` rule (D-024): `reference_name`, `canonical_reference`, variable and path variable name checks. |
| `model::id` | `RequestId`, `EnvironmentId`: UUID v4, one accepted spelling. |
| `model::order` | `OrderKey`: validated fractional-index key, `first()`, `between()` and `sequence()` to generate keys (D-038). |
| `model::request` | `Request`, `Method`, `QueryParam`, `PathParam`, `Header`, `Body`, `Part`, `RequestSettings`. |
| `model::local` | `Request::split()` → (`CommittedRequest`, `RequestLocalValues`) and `CommittedRequest::merge()`. `CommittedRequest` can only be built by `split()`, and it is the only type the file writer accepts (D-033). `LocalValues`: every request's local values by id (`.damnhttp/values.yaml`). |
| `model::auth` | `Auth`. |
| `model::variable` | `Variable`, `Environment`. |
| `model::collection` | `Workspace`, `Collection`, `Folder`: each item's own settings. The tree is the directory structure and belongs to the workspace store. |
| `model::validation` | `ValidationError`; every `validate()` returns all problems found. |

Differences from the table below, decided while implementing: form bodies use `Part` (text or file; a file in a urlencoded body is a validation error); `RequestSettings` fields are optional, `None` meaning the app default; `Request` carries `skip_tls_verify`, which `split()` moves to the local values.

| Type | Fields |
|---|---|
| `Workspace` | `name`, collections, environments |
| `Collection` | `name`, `order`, `description`, `variables: Vec<Variable>`, `auth: Auth`, children |
| `Folder` | `name`, `order`, `description`, `auth: Auth`, children |
| `Request` | `id`, `name`, `order`, `description`, `method`, `url`, `path_params: Vec<PathParam>`, `query: Vec<KeyValue>`, `headers: Vec<KeyValue>`, `auth: Auth`, `body: Body`, `settings: RequestSettings` |
| `KeyValue` | `name`, `value`, `enabled` (default true), `description`. In `query`, `value` is committed only if it is a `{{var}}` reference (D-021). |
| `PathParam` | `name`, `value`, `description`. Same value rule as `query`. Names are derived from `url`; the file lists only entries with a reference value or a description (D-028). |
| `LocalValues` | per request `id`: literal query and path variable values, `skip_tls_verify`. Stored in `.damnhttp/`, never committed. |
| `Body` | `None` \| `Json(text)` \| `Text(text, content_type)` \| `FormUrlencoded(Vec<KeyValue>)` \| `Multipart(Vec<Part>)`; a `Part` is a text value or a file path relative to the workspace |
| `Auth` | `Inherit` \| `None` \| `Basic { username, password }` \| `Bearer { token }`. Tagged enum, so OAuth2 / API key are new variants. Default is `Inherit` on folders and requests, `None` on collections. |
| `RequestSettings` | `follow_redirects`, `max_redirects`, `timeout_ms` |
| `Environment` | `id`, `name`, `variables: Vec<Variable>` |
| `Variable` | `name`, `value`, `secret`, `description`. A secret variable never carries a value in a committed file. |
| `ResolvedRequest` | fully interpolated method, URL, headers, body + effective `HttpOptions` (settings + local proxy / CA / TLS verification) |
| `ResponseMeta` | `response_id`, `status`, `status_text`, `http_version`, `headers`, `size { headers, body }`, `timing { total, dns+connect, ttfb, download }`, `content_type`, `redirects` |
| `HistoryEntry` | timestamp, request snapshot, `ResponseMeta`. Local only. |

Requests and environments have a stable `id` (UUID v4, generated once), because local state (literal values, secrets) must keep pointing at them when a teammate renames or moves the file (D-022). Folders and collections are identified by path.

## File format (`core::format`)

The spec is [`docs/FORMAT.md`](../FORMAT.md); JSON Schemas are in `schemas/v1/`; canonical examples are in `crates/core/tests/fixtures/format/`. The format is version 1. This section covers the code.

| Module | Contents |
|---|---|
| `format::emit` | The deterministic writer (D-008, D-036): a `Value` tree rendered by one function. Quoting rules, literal blocks, empty collections left out. |
| `format::tree` | Parses text with `saphyr-parser` (D-034) into `Node`s (scalars keep plain/quoted, entries keep key positions). Rejects anchors, aliases, tags, several documents, duplicate keys, nesting deeper than 32. |
| `format::read` | `MapReader`: typed, schema-driven access to a mapping; `finish()` rejects unknown keys (D-035). |
| `format::common` | File header (`id`, `version`, `kind`; version checked before anything else), auth, variables. |
| `format::request`, `containers`, `environment`, `values` | `write_*` / `read_*` per kind of file. |
| `format::error` | `FormatError { line, column, kind }`. Messages never contain values. |

Public API: `write_workspace` / `read_workspace`, `write_collection` / `read_collection`, `write_folder` / `read_folder`, `write_request(&CommittedRequest)` / `read_request -> RequestFile { request, literals }`, `write_environment` / `read_environment -> EnvironmentFile { environment, secret_values_in_file }`, `write_local_values` / `read_local_values` (`.damnhttp/values.yaml`, `LocalValues`), `FORMAT_VERSION`.

- The functions work on text only. Paths, slugs, directories, `.gitignore` and duplicate-id repair are the workspace store's job.
- `read_request` goes through `Request::split`, so literals written into a request file by hand come back in `RequestFile::literals`; the store merges them into `values.yaml` and reports them.
- `write_environment` never writes the value of a secret variable. Until `m2-secrets` adds a type for that split, the value of a secret variable read by hand from a file stays in the `Environment` and its name is reported in `secret_values_in_file`.
- Reading checks structure only; `validate()` on the model checks the rest, so a file with a semantic problem still opens.

## Workspace store (`core::store`)

`WorkspaceStore` is a workspace on disk. It is synchronous and has no UI or Tauri concerns; the `app` crate will wrap one call per command.

| Module | Contents |
|---|---|
| `store` | `WorkspaceStore`: `create`, `open`, `workspace`, `tree`, `create_collection` / `_folder` / `_request`, `read_*` / `save_*`, `rename`, `duplicate`, `move_to`, `delete`, `clean_local_values`, `repair_duplicate_ids`. |
| `store::tree` | `Tree { collections, problems }`, `TreeNode`, `TreeNodeKind`, `TreeProblem`. |
| `store::slug` | `slugify`, collision suffixes. |
| `store::fsutil` | Atomic writes (temporary file + rename, skipped when the content is unchanged), `.gitignore` entry. |
| `store::error` | `StoreError`. |

Rules it enforces (D-038):

- Items are named by workspace-relative paths with `/`, as returned by `tree()`. A path must stay under `collections/` and use plain components; symbolic links are never followed.
- `save_request` splits the request: committed part to the request file, local part to `.damnhttp/values.yaml`. `read_request` merges them. Local values are read before anything is written, so a corrupt `values.yaml` stops the save without losing data.
- A broken file is listed in `Tree::problems` and does not hide the rest of the tree.
- Environments are not handled yet (`m2-environments`).

## HTTP engine (`damnhttp-http`)

`send(&HttpRequest, &HttpOptions, body_path)` performs one exchange with `reqwest` over `rustls` and returns an `HttpResponse`. `send_cancellable` adds a cancel signal. No Tauri, no UI (D-041).

| Type | Contents |
|---|---|
| `HttpRequest` | `method`, `url` (fully resolved, query included), `headers`, `body: HttpBody` (`None`, `Raw`, `FormUrlencoded`, `Multipart` with text and file fields). |
| `HttpOptions` | `follow_redirects`, `max_redirects`, `timeout`, `verify_tls`, `proxy: ProxyMode` (`System`, `Disabled`, `Url`), `extra_ca_pem`. |
| `HttpResponse` | `status`, `status_text`, `http_version`, `headers`, `final_url`, `redirects`, `body_path`, `body_size`, `headers_size`, `timing { time_to_headers, download, total }`, `remote_address`. |
| `HttpError` | `kind: HttpErrorKind` (for the UI to translate) and `detail` (diagnostic text without URL, header values or body). |

- The response body is streamed to `body_path`; the caller chooses the file and deletes it when done. A failed or cancelled request removes the partial file.
- The engine refuses a URL that still contains `{{variables}}`. Turning a `core` `Request` into an `HttpRequest` (path variables, query, headers, body, later variables and auth) is the job of `m1-send-and-response`.
- A new client is built for every request, because proxy, CA and TLS settings can differ per request.

## URL templates (`core::url`)

The one parser for what the user types in the URL bar (D-028, D-039). The file loader, the model and, through the `url_analyze` command, the UI all use it.

| Function | Purpose |
|---|---|
| `url::analyze` | Path variable spans, `{{reference}}` spans (with validity), query and fragment ranges. Byte offsets. |
| `url::split_query` / `join_query` | Take the query string out of a typed URL, and put enabled parameters back for display. Nothing is decoded or encoded. |
| `url::substitute_path_variables` | Replace `:name` with percent-encoded values; report the ones without a value. |

`model::url_edit` adds the request-level operations: `Request::set_url_bar_text` (split the query, follow the URL with the path variable table, detect a rename), `url_bar_text`, `align_path_params`, `orphan_path_params`, `move_query_out_of_url`. `Request::split` calls `move_query_out_of_url`, and the store aligns path variables on load (keeping orphans) and on save (dropping them).

The `url_analyze` command must convert the byte offsets to UTF-16 offsets for the webview.

### Path variables and `{{var}}`

- A path variable is a whole path segment `:name` (as in Postman, D-040) in the path part of the URL (before `?` and `#`, not in the authority), with `name` matching `[A-Za-z_][A-Za-z0-9_.-]*`. So `/files/:name.pdf` has the variable `name.pdf`. Text inside `{{ }}` is skipped, and a segment containing a reference is not a variable. `localhost:8080`, `http://`, `/a:b`, `?t=:x`, `/:id{{x}}` and `{{a:b}}` contain none. One parser in `core` implements this for the file loader and, through `url_analyze`, for the UI (D-028).
- `{{name}}` is a variable reference anywhere in URL, query, headers, body and auth fields. `name` matches `[A-Za-z0-9_.-]+`; spaces inside the braces are allowed and ignored.
- Resolution order: path variables are substituted into the URL template first (their values may contain `{{var}}` and are percent-encoded after interpolation), then `{{var}}` is interpolated. A variable's value may reference other variables; cycles are reported, not followed.
- Precedence: environment variables override collection variables (D-014). Unresolved references are left as written and reported to the UI.

## Tauri commands

Naming: `<area>_<verb>`. Paths are workspace-relative strings. M1 and M2 commands are the plan; M3+ are provisional and will be settled when those milestones are planned.

| Milestone | Command | Purpose |
|---|---|---|
| M1 | `app_info` | version, platform, identifier |
| M1 | `settings_get`, `settings_set` | app settings (theme, language, proxy, CA, TLS verification) |
| M1 | `workspace_create`, `workspace_open`, `workspace_close` | open/create via a backend-side folder picker or a recent path |
| M1 | `workspace_recent_list` | recent workspaces |
| M1 | `workspace_tree` | the whole tree (names, kinds, methods, order) for the sidebar |
| M1 | `collection_create`, `folder_create`, `request_create` | new nodes |
| M1 | `node_rename`, `node_duplicate`, `node_move`, `node_delete` | tree operations; `node_move` covers drag and drop |
| M1 | `request_read`, `request_save` | request editor. Read merges the committed file with local values; save splits them (D-021). |
| M1 | `collection_read`, `collection_save`, `folder_read`, `folder_save` | collection / folder settings |
| M1 | `url_analyze` | the shared URL parser: path variable names, `{{var}}` spans, query split (D-028) |
| M1 | `local_data_clean` | "Clear unused local data": drop local values of requests that no longer exist |
| M1 | `http_send`, `http_cancel` | execute a saved request or an unsaved draft |
| M1 | `response_body` | read a byte range of a response body, raw or pretty-printed |
| M1 | `response_save_to_file` | backend-side save dialog |
| M2 | `environment_list`, `environment_read`, `environment_save`, `environment_create`, `environment_rename`, `environment_delete` | environments |
| M2 | `environment_set_active` | active environment (local state) |
| M2 | `secret_set`, `secret_clear`, `secret_status` | secret values; values are write-only from the UI except for an explicit reveal |
| M2 | `variables_resolve` | resolved variable map for a request: source scope, value (secrets masked), unresolved names |
| M2 | `rules_scan` | run the detection rules on a value or a request: credential findings and path-variable hints (D-026, D-029) |
| M2 | `value_share`, `value_make_local` | "Share with team" / "Make local again" (D-027) |
| M2 | `history_list`, `history_get`, `history_clear` | local history |
| M3 | `git_detect`, `git_check_remote`, `workspace_clone` | git detection, connection check, open from Git URL |
| M3 | `git_status`, `git_changes`, `git_save_version`, `git_sync`, `git_incoming` | simple sync UI and request-level diff |
| M3 | `git_log`, `git_raw_diff`, `git_branches`, `git_branch_create`, `git_branch_switch` | Advanced mode |
| M4 | `git_conflicts`, `git_resolve` | field-level conflict resolution |
| M4 | `leak_scan`, `leak_move_to_secret` | secret-leak guard on "Save version", built on the M2 rules module |
| M5 | `import_preview`, `import_apply` | importers with report |

Events (backend to frontend): `http:progress`, `workspace:changed` (files changed on disk), `git:updates` (background fetch result), `task:progress` (clone, sync, import).

## Git engine (outline; detailed at the start of M3)

```rust
trait GitEngine {
    // local: git2
    fn init / status / diff / commit / log / branches / read_conflict_stages / stage ...
    // network: system git
    fn detect / ls_remote / clone / fetch / pull_rebase / push ...
}
```

- `Git2Engine` implements local operations with vendored libgit2 built without network features.
- `SystemGit` runs the `git` executable non-interactively (`GIT_TERMINAL_PROMPT=0`, no stdin, timeouts, captured stderr) and maps stderr to typed errors.
- The app-facing engine composes both. With no system git, network methods return `GitError::GitNotInstalled` and everything else works.

## Frontend (planned)

- React 18+, Vite, TypeScript strict, Zustand stores per area (`workspace`, `tabs`, `environments`, `sync`, `settings`).
- CodeMirror 6 for URL bar, JSON/text bodies and the response viewer, with one shared extension for `{{var}}` highlighting, autocomplete and hover.
- CSS variables for theming (dark/light), CSS modules, no heavy component kit. `i18next` for strings. `dnd-kit` for the tree.
- Tests: Vitest + Testing Library for components, one Playwright smoke test.
