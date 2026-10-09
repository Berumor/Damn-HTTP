# Contributing to Damn HTTP

Thanks for your interest. The project is in early development, so please open an issue before starting anything large.

## Before you start

- Read [`docs/project/CONVENTIONS.md`](docs/project/CONVENTIONS.md) for code style, tests and the commands to build and run.
- Read [`docs/project/ARCHITECTURE.md`](docs/project/ARCHITECTURE.md) for the structure.
- Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Workflow

1. Fork the repository and create a branch from `develop`: `feature/<short-task>` or `fix/<short-task>`.
2. Make focused commits. Every commit message follows [Conventional Commits](https://www.conventionalcommits.org/) and is signed off (see below).
3. Open a pull request **to `develop`**. Its title is also a Conventional Commit message, for example `feat: add multipart body editor`.
4. Fill in the pull request template: what changed, why, how you tested it, and screenshots for UI changes.

Pull requests are merged with a merge commit, not squashed, so every commit you make reaches the release history. Version numbers and release notes are generated from commit messages:

| Type | Use it for | Effect on the next release |
|---|---|---|
| `feat:` | a new user-visible feature | minor version, listed in the notes |
| `fix:` | a user-visible bug fix | patch version, listed in the notes |
| `docs:`, `test:`, `refactor:`, `chore:`, `ci:` | everything else | none |

Add `!` after the type (`feat!:`) or a `BREAKING CHANGE:` footer for a breaking change. A correction to something you added earlier in the same branch is not a `fix:`; use `refactor:`, `test:` or `chore:`.

`main`, `beta` and `develop` are protected. Nobody pushes to them directly.

## Sign-off (DCO), no CLA

This project uses the [Developer Certificate of Origin](https://developercertificate.org/) instead of a contributor license agreement. By signing off a commit you state that you wrote the change or otherwise have the right to submit it under the project's license (Apache-2.0).

Sign off every commit with `-s`:

```
git commit -s -m "fix: handle empty header names"
```

This adds a line with the name and email from your git configuration:

```
Signed-off-by: Your Name <you@example.com>
```

If you forgot, `git commit --amend -s` fixes the last commit and `git rebase --signoff develop` fixes a whole branch that only you work on. A check on every pull request verifies the sign-off.

## What we will not merge

- Telemetry or analytics of any kind.
- Anything that makes the app store or ask for git credentials.
- Dependencies whose license is not compatible with Apache-2.0.

## Reporting bugs and asking for features

Use the issue templates. For security problems, do not open a public issue; see [`SECURITY.md`](SECURITY.md).
