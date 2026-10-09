# Agent instructions

Before doing anything else, read every file in `docs/project/`, starting with `STATUS.md` (where to resume) and `BRIEF.md` (the requirements; never rewrite it).

## Rules

- `docs/project/` is the single source of truth for project state. Update `STATUS.md`, `ROADMAP.md` and any other affected file at the end of every task, in the same PR as the code.
- Record every non-obvious choice in `docs/project/DECISIONS.md`. Ask the maintainer before deciding anything ambiguous and expensive to reverse (file format, git UX flows, release flow).
- No telemetry. Do not buy or register anything. Things that need an external account or setting go in `docs/project/NOTES.md`.

## Git workflow (summary; details in `docs/project/CONVENTIONS.md`)

- Long-lived branches: `main` (stable), `beta` (pre-releases), `develop` (integration). **Never commit or push to them directly.**
- Work on `feature/<task>` or `fix/<task>`, branched from an up-to-date `develop`. One branch = one focused task.
- Commits: Conventional Commits, signed off (`git commit -s`). PRs are merged with merge commits, never squashed, so every commit message reaches `main` and the release notes.
- When done: push, open a PR to `develop` (`gh pr create --base develop`). The PR title is a Conventional Commit message. The description covers what, why, how tested, screenshots for UI changes, decisions recorded.
- **Never merge PRs.** The maintainer reviews and merges. If the next task depends on an open PR, stop and say so.
- Never force-push shared branches or rewrite pushed history.
