# Notes

## Open questions for the maintainer

Raised in the `project-plan` PR. Each one is expensive to change once M1 is built.

1. **Workspace shape (D-005).** One repo = one workspace holding several collections, with environments shared across the workspace. OK, or one collection per repo, or environments per collection?
2. **Ordering (D-007).** A fractional `order` key in each file (no conflicts, one-file moves, not human-meaningful) versus an ordered child list in the parent file (readable, conflicts when two people add to the same folder).
3. **Secrets default (D-010).** OS keychain first with a git-ignored file as fallback, or always the git-ignored file? Should collection variables also be allowed to be secret?
4. **Proxy / custom CA (D-011).** Kept as local settings and never committed. The brief says "per request"; is a per-request proxy override needed in the v1 UI?
5. **libgit2 license (D-004).** GPL-2.0 with a linking exception. Fine for Apache-2.0 binaries; confirm you accept it, otherwise local git moves to `gix`.
6. **Names.** Manifest `damnhttp.yaml`, local directory `.damnhttp/`, extension `.yaml`. Any preference?

## Things the maintainer needs to do outside the repo

- [ ] **Install the toolchain on this machine (CachyOS).** On 2026-10-09 neither Rust nor Node was installed, so nothing can be built or tested locally yet. Needed before M0 code:
  `sudo pacman -S --needed rustup nodejs pnpm webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg xdotool` then `rustup default stable`. Later also `cargo-deny` (`cargo install cargo-deny`) and Playwright browsers.
- [ ] **Repository settings** (full list will be in `docs/RELEASING.md`, M7). Useful now: allow squash merging and merge commits; after `m0-ci` merges, protect `main`, `beta`, `develop` and require the CI checks.
- [ ] **Real exports for importer fixtures (M5).** Redistributable, with secrets removed: a large real Postman v2.1 collection plus its environments, an Insomnia v4 export, a Bruno collection, and any internal OpenAPI / Swagger specs that have caused trouble in other tools. A precise list will be written when M5 starts.

## Platform findings

Nothing yet. `m1-packaging-check` records AppImage / WebKitGTK results on CachyOS here.

Known risks to check then:

- AppImages built on `ubuntu-22.04` bundle an older WebKitGTK; on rolling distros this has caused blank windows or crashes with newer Mesa / NVIDIA drivers (workarounds involve `WEBKIT_DISABLE_DMABUF_RENDERER=1`). Verify, do not assume.
- AppImage needs FUSE (`libfuse2`) on some distros.

## Gotchas

- A Cargo package must not be named `core` (D-001).
- `git2` must be built without its network features so the build never pulls OpenSSL (D-004).
- The `project-plan` PR has no CI checks; CI arrives with `m0-ci`.
