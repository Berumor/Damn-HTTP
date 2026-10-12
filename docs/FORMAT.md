# Damn HTTP file format, version 1

A Damn HTTP workspace is a folder of plain YAML files, designed to be kept in git: one file per request, clean diffs, rare merge conflicts, and nothing in a committed file that belongs only to one machine.

This is the first version of the spec. It is complete for the files the app reads and writes today; it becomes the finished spec in milestone M6. JSON Schemas for every file are in [`schemas/v1/`](../schemas/v1/), and canonical examples in [`crates/core/tests/fixtures/format/`](../crates/core/tests/fixtures/format/).

## Workspace layout

```
my-workspace/
├── damnhttp.yaml              workspace manifest
├── .gitignore                 contains /.damnhttp/
├── .damnhttp/                 local only, never committed
│   └── values.yaml            literal query / path variable values, local flags
├── environments/
│   ├── dev.yaml
│   └── staging.yaml
└── collections/
    └── petstore/
        ├── collection.yaml
        ├── list-pets.yaml     a request
        └── pets/
            ├── folder.yaml
            └── get-pet-by-id.yaml
```

- A workspace holds one or more collections. A collection or folder is a directory with a `collection.yaml` or `folder.yaml`; every other `.yaml` file in it is a request. Folders nest.
- Environments are shared by every collection in the workspace.
- File and directory names are a slug of the item's `name` (`Get pet by ID` → `get-pet-by-id.yaml`). The name shown in the app is always the `name` key, never the file name.
  - A slug is lower-case ASCII letters and digits with words joined by `-`, at most 60 characters. Common accented Latin letters become their base letter (`Città` → `citta`); every other character separates words. A name with no usable character becomes `untitled`.
  - On a collision the slug gets a numeric suffix (`pets-2`). Names are compared without case and across files and directories, so a workspace behaves the same on every operating system.
  - Names that are reserved on Windows (`con`, `nul`, ...) and the names `collection` and `folder` get the suffix `-item`.
  - The app never follows symbolic links inside a workspace, and ignores files and directories whose name starts with a dot.
- `.damnhttp/` holds local state only: values that must not be committed, secrets, history, UI state. The app keeps `/.damnhttp/` in `.gitignore`.

## General rules

### Every file

- UTF-8, LF line endings, two-space indentation, exactly one trailing newline, no byte order mark.
- Every file starts with `version` and `kind`. Request and environment files start with `id` before them.
- `version` is the format version, `1` for this spec.
- `kind` is one of `workspace`, `collection`, `folder`, `request`, `environment`, `values`. A file of the wrong kind is refused (a `folder.yaml` cannot be read as a request).

### Determinism

The app writes files so that the same content always gives the same bytes, and an edit changes only the lines it is about.

- Keys are written in the fixed order given in the tables below, never alphabetically or in insertion order.
- Defaults are left out: `enabled: true`, `auth` when it is the default, empty strings, empty lists and maps, settings that use the app default.
- Lists (`query`, `headers`, `variables`, `fields`, ...) keep the order the user gave them.
- No timestamps, no per-save ids, no "last modified by".

### How values are written

The app writes a small subset of YAML: block mappings, block lists, and three kinds of scalar.

- **Plain** (`name: Get pet`) when no YAML 1.1 or 1.2 reader could take the text for anything other than the same string.
- **Double-quoted** (`url: "{{baseUrl}}/pets"`) otherwise. This covers text that would read as a boolean, null or number (`"true"`, `"no"`, `"8080"`, `"1.5"`, `""`), text starting with a YAML indicator (`{`, `[`, `-`, `*`, `&`, `!`, `#`, quotes, ...), text containing `: ` or ` #`, leading or trailing spaces, tabs, and invisible or control characters, which are escaped (`\t`, `\r`, `\x07`, ` `). Ids (UUIDs) are written plain even when they start with a digit.
- **Literal block** (`|`) for text with line breaks. `|` means the text ends with one line break, `|-` none, `|+` several. When the first line starts with a space the header carries an indentation indicator (`|2-`). Text containing a carriage return or a character that needs escaping is double-quoted instead, so it is kept exactly.

No flow collections (`[a, b]`, `{a: b}`), anchors, aliases, tags or comments are written.

### Reading files edited by hand

Files are meant to be edited by the app, but a file edited by hand, in any editor, still loads:

- Any YAML 1.2 style is accepted: single quotes, flow collections, folded blocks, CRLF line endings, a byte order mark, keys in any order. The app rewrites the file in the canonical form the next time it saves it.
- String keys take the text of any scalar: `value: 8080` and `name: no` are the strings `8080` and `no`. Boolean keys accept only plain `true` or `false`, number keys only plain digits.
- An empty value, `~` or `null` counts as the key being absent.
- These are errors, reported with a line and column: invalid YAML, an unknown key, a missing required key, a value of the wrong type, a duplicate key, anchors, aliases, tags, several documents in one file. Error messages name keys and positions, never values, because a value may be a secret.
- Semantic problems (an empty name, a duplicate variable, a file field in a URL-encoded body) do not stop a file from loading; the app shows them on the item.

### Versioning

- A file with a newer `version` than the app supports is refused with a message asking to update the app. The app never rewrites it, so an older app cannot drop fields it does not know. A team should update the app together when the format version changes.
- When a later version of the format exists, the app reads older versions and writes the current one.

### Ids

Requests and environments have an `id`: a UUID v4, lowercase and hyphenated, generated once when the item is created and never changed, written as the first line of the file. Local data (literal values, secrets) is keyed by it, so it survives a teammate renaming or moving the file. Folders and collections have no id; their identity is their path.

If two files share an id (a file copied by hand), the copy gets a new id: the copy is the file that is untracked or newer in git, or else the one whose path sorts later. "Duplicate" in the app always creates a new id and copies the local values.

### Ordering

Collections, folders and requests carry an `order` key: a short string of `0-9 A-Z a-z` (a fractional index). Siblings sort by `order` in byte order, then by file name. Moving an item rewrites one key in one file, and two people adding items to the same folder never conflict. When the keys of a sibling group grow too long, the app rewrites that group with short keys.

Keys the app generates have an integer part whose first character gives its length (`a0`, `a1`, ..., `az`, `b00`, ...) followed by an optional fraction (`a0V` sorts between `a0` and `a1`). Any other string of `0-9 A-Z a-z` written by hand is still read and sorted. The app rewrites a whole sibling group with fresh keys (`a0`, `a1`, ...) only when it has to place an item and cannot: the neighbours have equal keys, a neighbour's key does not follow this layout, or the new key would be longer than 12 characters.

### Committed and local values

Some values must never reach a committed file.

- **Query parameters and path variables** (D-021). A request file holds their names, `enabled` flags, order and descriptions. A `value` is written only when, after trimming, it is exactly one variable reference, `{{name}}` (spaces inside the braces are allowed and removed when written). Anything else, including `{{a}}-1` or `{{a}}{{b}}`, is a literal and is stored in `.damnhttp/values.yaml`, keyed by request id. A teammate who clones the workspace sees the same parameters with empty values. A literal found in a request file (a hand edit) is moved to the local values when the file is loaded, and the next save writes the file without it.
- **Secret variables** (D-019). Only environment variables can be secret. A secret variable is written as its name and `secret: true`, never with its value; the value is stored in `.damnhttp/`. A value found on a secret variable (a hand edit) is reported, because it is already in the repository history, and the next save removes it from the file.
- **Machine-specific settings** (D-020): proxy, custom CA and "skip TLS verification" are never in committed files. "Skip TLS verification" is stored per request in `.damnhttp/values.yaml`.

Header values, bodies and form fields are committed as written (D-025). Before "Save version" the app scans them for values that look like credentials and offers to move them into a secret variable.

**Limitation (D-026).** Literal text in the URL path or host is committed as written: in `/it/stations/1234` the app cannot tell data from fixed path text. Use a path variable (`/:country/stations/:id`) for data. The app suggests this for path segments that look like ids.

### Path variables and `{{variables}}`

- A **path variable** is `:name` where the `:` starts a path segment (it directly follows `/`) in the path part of the URL, and `name` matches `[A-Za-z_][A-Za-z0-9_]*`. Text inside `{{ }}` is skipped. So `localhost:8080`, `http://`, `/a:b`, `?t=:x` and `{{a:b}}` contain no path variable. A literal segment starting with `:` is written percent-encoded (`%3A`).
- A **variable reference** is `{{name}}` with `name` matching `[A-Za-z0-9_.-]+`. It can appear in the URL, query, headers, body and auth fields.
- When a request is sent, path variables are substituted into the URL first (their values may contain references and are percent-encoded after interpolation), then references are interpolated. Environment variables override collection variables.

## Files

In the tables, keys are listed in the order they are written. "Required" keys are always written; other keys are left out when they hold the default.

### Workspace manifest: `damnhttp.yaml`

| Key | Type | Required / default | Notes |
|---|---|---|---|
| `version` | number | required | `1` |
| `kind` | string | required | `workspace` |
| `name` | string | required | |

```yaml
version: 1
kind: workspace
name: Acme APIs
```

### Collection: `collections/<slug>/collection.yaml`

| Key | Type | Required / default | Notes |
|---|---|---|---|
| `version`, `kind` | | required | `kind: collection` |
| `name` | string | required | |
| `order` | string | required | see [Ordering](#ordering) |
| `description` | string | `""` | |
| `variables` | list of [variables](#variables) | empty | committed with their values; cannot be secret |
| `auth` | [auth](#auth) | `none` | default auth for everything in the collection |

```yaml
version: 1
kind: collection
name: Petstore
order: a0
description: The public Petstore API.
variables:
  - name: apiVersion
    value: v2
  - name: pageSize
    value: "50"
    description: Default page size for list endpoints.
auth:
  type: bearer
  token: "{{apiToken}}"
```

### Folder: `folder.yaml`

| Key | Type | Required / default | Notes |
|---|---|---|---|
| `version`, `kind` | | required | `kind: folder` |
| `name` | string | required | |
| `order` | string | required | |
| `description` | string | `""` | |
| `auth` | [auth](#auth) | `inherit` | |

### Request: `<slug>.yaml`

| Key | Type | Required / default | Notes |
|---|---|---|---|
| `id` | string | required | UUID v4, first line |
| `version`, `kind` | | required | `kind: request` |
| `name` | string | required | |
| `order` | string | required | |
| `description` | string | `""` | |
| `method` | string | required | `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS` or any HTTP method token; case-sensitive |
| `url` | string | required | URL template without a query string; may be `""` |
| `path_params` | list | empty | only path variables with a reference value or a description are listed; the names come from `url` |
| `query` | list | empty | |
| `headers` | list | empty | |
| `auth` | [auth](#auth) | `inherit` | |
| `body` | [body](#body) | `none` | |
| `settings` | mapping | app defaults | |

`path_params` items: `name` (required), `value` (a reference only), `description`.

`query` items: `name` (required), `value` (a reference only), `enabled` (default `true`), `description`. Repeated names are allowed; their local values are matched by position among parameters of the same name.

`headers` items: `name` (required), `value`, `enabled` (default `true`), `description`.

`settings` keys, each left out when it uses the app default: `follow_redirects` (boolean), `max_redirects` (number), `timeout_ms` (number).

```yaml
id: 3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70
version: 1
kind: request
name: Update pet
order: a1V
description: |
  Updates a pet.

  The `petId` is the id returned by "Create pet".
method: PUT
url: "{{baseUrl}}/{{apiVersion}}/stores/:storeId/pets/:petId"
path_params:
  - name: storeId
    value: "{{storeId}}"
  - name: petId
    description: The pet to update.
query:
  - name: verbose
    enabled: false
  - name: tenant
    value: "{{tenantId}}"
    description: Required by the multi-tenant gateway.
headers:
  - name: Accept
    value: application/json
auth:
  type: none
body:
  type: json
  content: |
    {
      "name": "Rex"
    }
settings:
  timeout_ms: 30000
```

#### Body

`type` is required and decides the other keys:

| `type` | Other keys |
|---|---|
| `none` | none (the same as leaving `body` out) |
| `json` | `content` |
| `text` | `content`, `content_type` (absent means `text/plain`) |
| `form-urlencoded` | `fields`: items with `name` (required), `value`, `enabled` |
| `multipart` | `fields`: items with `name` (required), then either `value` or `file`, and `enabled` |

A `file` is a path relative to the workspace root. A field cannot have both `value` and `file`.

### Environment: `environments/<slug>.yaml`

| Key | Type | Required / default | Notes |
|---|---|---|---|
| `id` | string | required | UUID v4, first line |
| `version`, `kind` | | required | `kind: environment` |
| `name` | string | required | |
| `variables` | list of [variables](#variables) | empty | may be secret |

```yaml
id: b81d0c34-5e2f-4a67-8f19-6c3e9a7d2b05
version: 1
kind: environment
name: Staging
variables:
  - name: baseUrl
    value: https://staging.example.com
  - name: apiToken
    secret: true
    description: Ask the team lead for a staging token.
```

### Local values: `.damnhttp/values.yaml` (never committed)

| Key | Type | Notes |
|---|---|---|
| `version`, `kind` | | `kind: values` |
| `requests` | mapping | request id → local values, sorted by id; requests with nothing to store are left out |

Each request entry has `path_params` (path variable name → value), `query` (items with `name`, `occurrence`, `value`) and `skip_tls_verify` (default `false`). `occurrence` counts from 0 among query parameters with the same name, and is left out when it is 0.

```yaml
version: 1
kind: values
requests:
  3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70:
    path_params:
      petId: "42"
    query:
      - name: tag
        value: dog
      - name: tag
        occurrence: 1
        value: good
  9d4e2f7a-1b3c-4e5d-8f6a-0c2b4d6e8f10:
    skip_tls_verify: true
```

### Shared structures

#### Variables

Items with `name` (required, `[A-Za-z0-9_.-]+`), `value`, `secret` (written only when `true`), `description`. A secret variable has no `value` key.

#### Auth

`type` is required:

| `type` | Other keys | Meaning |
|---|---|---|
| `inherit` | none | use the parent's auth (the default for folders and requests) |
| `none` | none | send no credentials, even if a parent defines some (the default for collections) |
| `basic` | `username`, `password` | |
| `bearer` | `token` | |

Credentials are usually references (`token: "{{apiToken}}"`) to a secret environment variable.

## Editor validation

The schemas in `schemas/v1/` follow JSON Schema draft 2020-12. `file.schema.json` picks the right schema from `kind`, so one entry covers every file. With the YAML extension for VS Code, add to the workspace's `.vscode/settings.json`:

```json
{
  "yaml.schemas": {
    "https://raw.githubusercontent.com/Berumor/Damn-HTTP/main/schemas/v1/file.schema.json": [
      "damnhttp.yaml",
      "collections/**/*.yaml",
      "environments/*.yaml",
      ".damnhttp/values.yaml"
    ]
  }
}
```

The schemas are stricter than the app in one way: string keys expect YAML strings, so a hand-written `value: 8080` is flagged by the editor although the app reads it as the string `8080`. The schemas also flag literal query and path variable values and secret values in committed files, which the app would move out of the file.
