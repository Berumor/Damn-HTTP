# Notes

## Open questions for the maintainer

None. The questions from PR #1 and PR #2 are answered and recorded in `DECISIONS.md` (D-004, D-005, D-007, D-019 to D-029).

## Possible later features

- A per-field "keep local" option for header values and body / form fields, so they can stay out of committed files like query and path variable values do (D-025).
- "Share with team" into an environment variable; v1 is collection scope only (D-027).
- An OS keychain backend for secrets (D-019).

## Things the maintainer needs to do outside the repo

- [ ] **Install the toolchain on this machine (CachyOS).** On 2026-10-09 neither Rust nor Node was installed, so nothing can be built or tested locally yet. Needed before M0 code:
  `sudo pacman -S --needed rustup nodejs pnpm webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool` then `rustup default stable`. Later also `cargo-deny` (`cargo install cargo-deny`) and Playwright browsers.
- [ ] **Repository settings** (full list will be in `docs/RELEASING.md`, M7). Useful now: allow merge commits only, disable squash and rebase merging (D-023); after `m0-ci` merges, protect `main`, `beta`, `develop` and require the CI checks.
- [ ] **Real exports for importer fixtures (M5).** Redistributable, with secrets removed: a large real Postman v2.1 collection plus its environments, an Insomnia v4 export, a Bruno collection, and any internal OpenAPI / Swagger specs that have caused trouble in other tools. A precise list will be written when M5 starts.

## Platform findings

Nothing yet. `m1-packaging-check` records AppImage / WebKitGTK results on CachyOS here.

Known risks to check then:

- AppImages built on `ubuntu-22.04` bundle an older WebKitGTK; on rolling distros this has caused blank windows or crashes with newer Mesa / NVIDIA drivers (workarounds involve `WEBKIT_DISABLE_DMABUF_RENDERER=1`). Verify, do not assume.
- AppImage needs FUSE (`libfuse2`) on some distros.

## Gotchas

- A Cargo package must not be named `core` (D-001).
- `git2` must be built without its network features so the build never pulls OpenSSL (D-004).
- No PR has CI checks until `m0-ci` is merged.
- Commits are SSH-signed with a passphrase-protected key. If `git commit` fails with an `ssh_askpass` error, the maintainer has to load `~/.ssh/github_sign` into the agent from a real terminal. Do not bypass signing.
