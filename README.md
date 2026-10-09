# Damn HTTP

An open-source, git-native REST API client for teams. A desktop app for Windows, macOS and Linux, and an alternative to Postman, Insomnia and Bruno.

> **Status: early development.** There is nothing to download or run yet. The plan and its progress are in [`docs/project/`](docs/project/).

## What it is

Your API collections are plain YAML files in a normal git repository. A team shares and updates them through git, and the app makes that usable by people who have never used git: "Save version" and "Sync" instead of commit, pull and push, a diff that shows which request changed and how, and conflict resolution field by field.

Planned for the first version:

- **Collections** of folders and requests, one small text file per request, written so diffs are clean and merge conflicts are rare.
- **Requests**: method, URL, query params, headers, bodies (JSON, text, form, multipart), path variables with the `:id` syntax.
- **Variables and environments** with `{{var}}` interpolation, and **secrets** that never leave your machine.
- **Local values**: the literal values you type into query params and path variables stay on your machine; only their names are shared.
- **A secret-leak guard** that warns before a credential is saved into a shared file.
- **Git sync** through the `git` already installed on your system, using the credentials you already have. The app never asks for or stores git credentials.
- **Import** from Postman, Insomnia, Bruno and OpenAPI / Swagger.

Not planned for the first version: scripting, GraphQL, gRPC, WebSocket, cloud accounts. There is no telemetry of any kind, and there never will be.

## Built with

Tauri 2, Rust, React, TypeScript and CodeMirror 6. All HTTP requests are sent from the Rust backend.

## Documentation

- [`docs/project/BRIEF.md`](docs/project/BRIEF.md): what is being built and why.
- [`docs/project/ROADMAP.md`](docs/project/ROADMAP.md): milestones and their state.
- [`docs/project/ARCHITECTURE.md`](docs/project/ARCHITECTURE.md): structure, data model and the file format draft.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Commits must be signed off (`git commit -s`); there is no CLA. Please read the [Code of Conduct](CODE_OF_CONDUCT.md). To report a vulnerability, see [`SECURITY.md`](SECURITY.md).

## License

[Apache License 2.0](LICENSE). See also [`NOTICE`](NOTICE).
