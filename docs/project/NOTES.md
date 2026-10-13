# Notes

## Open questions for the maintainer

None. The questions from PR #1 and PR #2 are answered and recorded in `DECISIONS.md` (D-004, D-005, D-007, D-019 to D-029).

## Possible later features

- Separate DNS / connect / TLS timings in the response viewer (needs a custom connector, D-041).
- Connection reuse between requests, and zstd response decompression (D-041).
- A per-field "keep local" option for header values and body / form fields, so they can stay out of committed files like query and path variable values do (D-025).
- "Share with team" into an environment variable; v1 is collection scope only (D-027).
- An OS keychain backend for secrets (D-019).

## Things the maintainer needs to do outside the repo

- [x] **Toolchain on this machine (CachyOS).** Installed 2026-10-09: rustup (Rust 1.99.0), Node 26, pnpm 11, webkit2gtk-4.1, cargo-deny (via `cargo install`). Still to come: Playwright browsers (`m1-smoke-test`).
- [ ] **Repository settings** (full list will be in `docs/RELEASING.md`, M7). Useful now: allow merge commits only, disable squash and rebase merging (D-023); turn on "Automatically delete head branches" so merged branches are removed; after `m0-ci` merges, protect `main`, `beta`, `develop` and require these checks: "Rust (fmt, clippy, tests)", "Frontend (lint, typecheck, tests, build)", "cargo-deny (licenses, advisories)", "PR title (Conventional Commits)", "Commits (Conventional Commits, DCO sign-off)". Do not require approvals while there is a single maintainer account (the PR author cannot approve their own PR).
- [ ] **Before the first release on `main`** (maintainer's decision, 2026-10-09: deferred until then): the next two items.
- [ ] **Enable private vulnerability reporting** (repository Settings → Security). `SECURITY.md` and the issue chooser point reporters to it.
- [ ] **Give a contact for the Code of Conduct and security reports.** `CODE_OF_CONDUCT.md` and `SECURITY.md` currently say "the contact details on @Berumor's GitHub profile". Supply an email address to print there, or make sure the profile shows one.
- [ ] **A real app icon.** The current one is a placeholder (D-042). The maintainer will ask their designer (2026-10-13). A 1024×1024 PNG is enough.
- [ ] **Real exports for importer fixtures (M5).** Redistributable, with secrets removed: a large real Postman v2.1 collection plus its environments, an Insomnia v4 export, a Bruno collection, and any internal OpenAPI / Swagger specs that have caused trouble in other tools. A precise list will be written when M5 starts.

## Platform findings

- 2026-10-13, CachyOS, KDE Plasma on Wayland, WebKitGTK 2.54.1: a debug build (`pnpm tauri build --debug --no-bundle`) starts, renders correctly, follows the system dark theme and reaches the backend over IPC. No workaround needed. Screenshot: `docs/screenshots/m1-tauri-skeleton.png`.
- The window shows a generic icon in the title bar when run from `target/`; an installed build gets its icon from the desktop file.
- 2026-10-13, first packaging run (workflow run 38451177211): all four targets build with no extra setup. `aws-lc-sys` needed nothing special on Windows or macOS. Sizes: AppImage 81 MB (it bundles WebKitGTK), macOS `.dmg` 2.9 MB, Windows NSIS installer 2.0 MB.
- 2026-10-13, the AppImage built on `ubuntu-22.04`, run on CachyOS (KDE Plasma, Wayland): it starts and keeps running with no error output. The machine has `fuse3` only, no `fuse2` / `libfuse2`, and that was enough. **Not yet confirmed by eye** that the window renders (the blank-window risk below); the maintainer was asked to check.
- The Intel macOS build is cross-compiled on an Apple Silicon runner (`--target x86_64-apple-darwin`); it has not been run on real Intel hardware.
- Nobody has run the macOS or Windows installers yet. They are unsigned: macOS Gatekeeper and Windows SmartScreen will warn (to be documented in the README in M7).

Known risks to check then:

- AppImages built on `ubuntu-22.04` bundle an older WebKitGTK; on rolling distros this has caused blank windows or crashes with newer Mesa / NVIDIA drivers (workarounds involve `WEBKIT_DISABLE_DMABUF_RENDERER=1`). Verify, do not assume.
- AppImage needs FUSE (`libfuse2`) on some distros.

## Gotchas

- A Cargo package must not be named `core` (D-001).
- Do not take "active window" screenshots to check the app: on 2026-10-13 that captured the maintainer's browser instead (the file was deleted, never committed). Ask the maintainer to look, or capture only when the app window is known to be focused.
- Building the app crate on Linux needs: `webkit2gtk-4.1`, `libayatana-appindicator` (or `libappindicator-gtk3`), `librsvg`, `xdotool`, `openssl`, and a C toolchain. CI installs the Debian equivalents in the Rust job.
- The app icon in `crates/app/icons/` is a placeholder. To replace it: `pnpm tauri icon <1024px.png> -o crates/app/icons`, then delete the `android`, `ios`, `Square*` and `StoreLogo` files it also creates.
- pnpm 11 refuses packages published very recently unless an exception is written to `pnpm-workspace.yaml`. Do not add exceptions; pick a slightly older version (D-042).
- `crates/app/gen/` and `dist/` are build output and are git-ignored.
- `url::substitute_path_variables` leaves `{{references}}` inside a path variable value untouched. `m2-variable-engine` has to percent-encode what they resolve to (D-039).
- Query parameters are stored and sent as typed; only characters illegal in a URL are encoded (D-039, D-041).
- The HTTP tests start servers on `127.0.0.1` and one test resolves a `.invalid` host name; they need a working loopback interface and resolver, nothing else.
- `aws-lc-sys` (TLS crypto, via reqwest) builds C code. If a Windows or macOS CI build fails on it, see the aws-lc-rs build requirements before changing crypto provider (D-041).
- `url::analyze` returns byte offsets. The `url_analyze` command (`m1-tauri-skeleton` / `m1-request-editor`) must convert them to UTF-16 offsets for JavaScript.
- Every session needs an SSH agent holding both `~/.ssh/github_auth` (push) and `~/.ssh/github_sign` (commit signing). Without it, `git fetch` over SSH fails; reading still works through `gh`.
- Merged branches are deleted on request (done on 2026-10-12 for everything up to PR #8). Cloud sessions can leave `ccr-*` branches on GitHub; delete them once their PR is merged.
- `duplicate` in the workspace store does not copy non-YAML files (D-038). Revisit when multipart file attachments get a UI in `m1-request-editor`.
- `NOTICE` must gain an entry for libgit2 (GPL-2.0 with linking exception) in the PR that adds `git2` (D-004), and for any other bundled third-party code.
- `CODE_OF_CONDUCT.md` is the Contributor Covenant 2.1 text, unmodified except for the contact sentence. `.gitignore` ignores `.env*`; nothing in this repo should need one.
- `git2` must be built without its network features so the build never pulls OpenSSL (D-004).
- CI exists from `m0-ci` on. PRs #1 to #5 were merged or opened without checks.
- `cargo-deny` is installed in `~/.cargo/bin`, which is not on the default `PATH` on this machine. Run it as `~/.cargo/bin/cargo-deny check` or add that directory to `PATH`.
- `saphyr-parser` reports 0-based columns although its docs say 1-based; `format::tree` adds 1. It also has its own recursion limit, which fires before ours on very deep flow nesting (D-034).
- The JSON Schema `$id` URLs point at `main`, so they resolve only after the first release that contains `schemas/` (D-037). Until then, point editors at the local files.
- Commits are SSH-signed with a passphrase-protected key. If `git commit` fails with an `ssh_askpass` error, the maintainer has to load `~/.ssh/github_sign` into the agent from a real terminal. Do not bypass signing.
