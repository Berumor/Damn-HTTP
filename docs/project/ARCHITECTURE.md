# Architecture

State: **plan only**. No application code exists yet. Update this file whenever the structure changes.

## Repository layout (planned)

```
Cargo.toml              Cargo workspace (version 0.0.0 placeholder, shared lints and deps)
rust-toolchain.toml     pinned stable toolchain
deny.toml               cargo-deny config (licenses compatible with Apache-2.0)
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
schemas/                JSON Schemas for the file format
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
collection /  ├─> core: resolve auth inheritance ─> substitute :path variables
folder chain  │         ─> interpolate {{vars}} (environment > collection) ─> ResolvedRequest
environment ──┤
local secrets ┘
ResolvedRequest ─> http: execute ─> ResponseMeta (+ body written to a temp file)
```

Response bodies are not serialized into the JSON result. `http_send` returns metadata plus a `response_id`; the frontend then reads the body in ranges as raw bytes. Pretty-printing JSON happens in Rust. This keeps multi-MB responses from freezing the webview (D-012).

## Data model (`core`)

| Type | Fields |
|---|---|
| `Workspace` | `name`, collections, environments |
| `Collection` | `name`, `order`, `description`, `variables: Vec<Variable>`, `auth: Auth`, children |
| `Folder` | `name`, `order`, `description`, `auth: Auth`, children |
| `Request` | `name`, `order`, `description`, `method`, `url`, `path_params: Vec<PathParam>`, `query: Vec<KeyValue>`, `headers: Vec<KeyValue>`, `auth: Auth`, `body: Body`, `settings: RequestSettings` |
| `KeyValue` | `name`, `value`, `enabled` (default true), `description` |
| `PathParam` | `name`, `value` |
| `Body` | `None` \| `Json(text)` \| `Text(text, content_type)` \| `FormUrlencoded(Vec<KeyValue>)` \| `Multipart(Vec<Part>)`; a `Part` is a text value or a file path relative to the workspace |
| `Auth` | `Inherit` \| `None` \| `Basic { username, password }` \| `Bearer { token }`. Tagged enum, so OAuth2 / API key are new variants. Default is `Inherit` on folders and requests, `None` on collections. |
| `RequestSettings` | `follow_redirects`, `max_redirects`, `timeout_ms`, `verify_tls` |
| `Environment` | `id`, `name`, `variables: Vec<Variable>` |
| `Variable` | `name`, `value`, `secret`, `description`. A secret variable never carries a value in a committed file. |
| `ResolvedRequest` | fully interpolated method, URL, headers, body + effective `HttpOptions` (settings + local proxy / CA) |
| `ResponseMeta` | `response_id`, `status`, `status_text`, `http_version`, `headers`, `size { headers, body }`, `timing { total, dns+connect, ttfb, download }`, `content_type`, `redirects` |
| `HistoryEntry` | timestamp, request snapshot, `ResponseMeta`. Local only. |

Identity is the file path. Only environments have an `id`, because local secrets must keep pointing at them across renames (D-006).

## File format (draft v1)

This is the draft to review. The finished spec will be `docs/FORMAT.md` with JSON Schemas in `schemas/` (task `m1-file-format`).

A workspace is a folder, normally a git repository root:

```
my-workspace/
├── damnhttp.yaml              workspace manifest
├── .gitignore                 the app adds /.damnhttp/ here
├── .damnhttp/                 git-ignored: secrets file, history, UI state, local HTTP settings
├── environments/
│   ├── dev.yaml
│   └── staging.yaml
└── collections/
    └── petstore/
        ├── collection.yaml
        ├── list-pets.yaml
        └── pets/
            ├── folder.yaml
            └── get-pet-by-id.yaml
```

`damnhttp.yaml`:

```yaml
version: 1
kind: workspace
name: Acme APIs
```

`collections/petstore/collection.yaml`:

```yaml
version: 1
kind: collection
name: Petstore
order: a0
variables:
  - name: apiVersion
    value: v2
auth:
  type: bearer
  token: "{{apiToken}}"
```

`collections/petstore/pets/get-pet-by-id.yaml`:

```yaml
version: 1
kind: request
name: Get pet by ID
order: a1
description: Returns a single pet.
method: GET
url: "{{baseUrl}}/{{apiVersion}}/pets/:petId"
path_params:
  - name: petId
    value: "42"
query:
  - name: verbose
    value: "true"
    enabled: false
headers:
  - name: Accept
    value: application/json
body:
  type: json
  content: |
    {
      "name": "Rex"
    }
settings:
  timeout_ms: 30000
```

`environments/staging.yaml`:

```yaml
version: 1
kind: environment
id: env_8f3k2q7d
name: Staging
variables:
  - name: baseUrl
    value: https://staging.example.com
  - name: apiToken
    secret: true
```

Rules:

- **Determinism.** Keys are written in a fixed order defined per `kind`, never alphabetically or by insertion. Same model in, byte-identical file out. UTF-8, LF, two-space indent, one trailing newline, no BOM. Scalars are plain unless a documented rule requires double quotes; multi-line text uses `|` block scalars. The emitter is our own code over this restricted YAML subset (D-008).
- **Defaults are omitted.** `enabled: true`, `auth: inherit`, empty lists, empty descriptions and default settings are not written, so files stay short and unrelated edits do not touch them.
- **No volatile data.** No timestamps, no per-save IDs, no "last modified by".
- **Names and files.** `name` holds the display name. The file or directory name is a slug of it (`get-pet-by-id.yaml`), with a numeric suffix on collision. Renaming in the app renames the file.
- **Ordering.** Each item carries a short `order` key (fractional index). Siblings sort by `order`, then by file name. Moving an item rewrites one field in one file; two people adding items to the same folder never conflict (D-007).
- **Lists** (`query`, `headers`, `variables`, ...) keep the user's order.
- **Versioning.** Every file has `version`. The app migrates older versions on write. A file with a newer version than the app supports opens read-only, so an old app never drops fields it does not know (D-009).
- **Secrets.** A variable with `secret: true` has no `value` key in the committed file. Its value lives in the OS keychain or `.damnhttp/secrets.yaml`, keyed by environment `id` + variable name.
- **Machine-specific HTTP settings** (proxy, custom CA path) live in `.damnhttp/`, not in request files (D-011).

### Path variables and `{{var}}`

- A path variable is `:name` where the `:` starts a path segment (it directly follows `/`), the segment is in the path part of the URL (before `?` and `#`, not in the authority), and `name` matches `[A-Za-z_][A-Za-z0-9_]*`. So `localhost:8080`, `http://`, `/a:b` and `?t=:x` contain none.
- `{{name}}` is a variable reference anywhere in URL, query, headers, body and auth fields. `name` matches `[A-Za-z0-9_.-]+`.
- Resolution order: path variables are substituted into the URL template first (their values may contain `{{var}}` and are percent-encoded after interpolation), then `{{var}}` is interpolated. A variable's value may reference other variables; cycles are reported, not followed.
- Precedence: environment variables override collection variables (D-014). Unresolved references are left as written and reported to the UI.

## Tauri commands

Naming: `<area>_<verb>`. Paths are workspace-relative strings. M1 and M2 commands are the plan; M3+ are provisional and will be settled when those milestones are planned.

| Milestone | Command | Purpose |
|---|---|---|
| M1 | `app_info` | version, platform, identifier |
| M1 | `settings_get`, `settings_set` | app settings (theme, language, proxy, CA) |
| M1 | `workspace_create`, `workspace_open`, `workspace_close` | open/create via a backend-side folder picker or a recent path |
| M1 | `workspace_recent_list` | recent workspaces |
| M1 | `workspace_tree` | the whole tree (names, kinds, methods, order) for the sidebar |
| M1 | `collection_create`, `folder_create`, `request_create` | new nodes |
| M1 | `node_rename`, `node_duplicate`, `node_move`, `node_delete` | tree operations; `node_move` covers drag and drop |
| M1 | `request_read`, `request_save` | request editor |
| M1 | `collection_read`, `collection_save`, `folder_read`, `folder_save` | collection / folder settings |
| M1 | `http_send`, `http_cancel` | execute a saved request or an unsaved draft |
| M1 | `response_body` | read a byte range of a response body, raw or pretty-printed |
| M1 | `response_save_to_file` | backend-side save dialog |
| M2 | `environment_list`, `environment_read`, `environment_save`, `environment_create`, `environment_rename`, `environment_delete` | environments |
| M2 | `environment_set_active` | active environment (local state) |
| M2 | `secret_set`, `secret_clear`, `secret_status` | secret values; values are write-only from the UI except for an explicit reveal |
| M2 | `variables_resolve` | resolved variable map for a request: source scope, value (secrets masked), unresolved names |
| M2 | `history_list`, `history_get`, `history_clear` | local history |
| M3 | `git_detect`, `git_check_remote`, `workspace_clone` | git detection, connection check, open from Git URL |
| M3 | `git_status`, `git_changes`, `git_save_version`, `git_sync`, `git_incoming` | simple sync UI and request-level diff |
| M3 | `git_log`, `git_raw_diff`, `git_branches`, `git_branch_create`, `git_branch_switch` | Advanced mode |
| M4 | `git_conflicts`, `git_resolve` | field-level conflict resolution |
| M4 | `leak_scan`, `leak_move_to_secret` | secret-leak guard |
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
