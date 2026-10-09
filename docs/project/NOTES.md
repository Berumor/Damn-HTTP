# Notes

## Open questions for the maintainer

None. The questions from PR #1 and PR #2 are answered and recorded in `DECISIONS.md` (D-004, D-005, D-007, D-019 to D-029).

## Possible later features

- A per-field "keep local" option for header values and body / form fields, so they can stay out of committed files like query and path variable values do (D-025).
- "Share with team" into an environment variable; v1 is collection scope only (D-027).
- An OS keychain backend for secrets (D-019).

## Things the maintainer needs to do outside the repo

- [x] **Toolchain on this machine (CachyOS).** Installed 2026-10-09: rustup (Rust 1.99.0), Node 26, pnpm 11, webkit2gtk-4.1, cargo-deny (via `cargo install`). Still to come: Playwright browsers (`m1-smoke-test`).
- [ ] **Repository settings** (full list will be in `docs/RELEASING.md`, M7). Useful now: allow merge commits only, disable squash and rebase merging (D-023); turn on "Automatically delete head branches" so merged branches are removed; after `m0-ci` merges, protect `main`, `beta`, `develop` and require the CI checks.
- [ ] **Before the first release on `main`** (maintainer's decision, 2026-10-09: deferred until then): the next two items.
- [ ] **Enable private vulnerability reporting** (repository Settings → Security). `SECURITY.md` and the issue chooser point reporters to it.
- [ ] **Give a contact for the Code of Conduct and security reports.** `CODE_OF_CONDUCT.md` and `SECURITY.md` currently say "the contact details on @Berumor's GitHub profile". Supply an email address to print there, or make sure the profile shows one.
- [ ] **Real exports for importer fixtures (M5).** Redistributable, with secrets removed: a large real Postman v2.1 collection plus its environments, an Insomnia v4 export, a Bruno collection, and any internal OpenAPI / Swagger specs that have caused trouble in other tools. A precise list will be written when M5 starts.

## Platform findings

Nothing yet. `m1-packaging-check` records AppImage / WebKitGTK results on CachyOS here.

Known risks to check then:

- AppImages built on `ubuntu-22.04` bundle an older WebKitGTK; on rolling distros this has caused blank windows or crashes with newer Mesa / NVIDIA drivers (workarounds involve `WEBKIT_DISABLE_DMABUF_RENDERER=1`). Verify, do not assume.
- AppImage needs FUSE (`libfuse2`) on some distros.

## Gotchas

- A Cargo package must not be named `core` (D-001).
- `NOTICE` must gain an entry for libgit2 (GPL-2.0 with linking exception) in the PR that adds `git2` (D-004), and for any other bundled third-party code.
- `CODE_OF_CONDUCT.md` is the Contributor Covenant 2.1 text, unmodified except for the contact sentence. `.gitignore` ignores `.env*`; nothing in this repo should need one.
- `git2` must be built without its network features so the build never pulls OpenSSL (D-004).
- No PR has CI checks until `m0-ci` is merged.
- `cargo-deny` is installed in `~/.cargo/bin`, which is not on the default `PATH` on this machine. Run it as `~/.cargo/bin/cargo-deny check` or add that directory to `PATH`.
- Commits are SSH-signed with a passphrase-protected key. If `git commit` fails with an `ssh_askpass` error, the maintainer has to load `~/.ssh/github_sign` into the agent from a real terminal. Do not bypass signing.
