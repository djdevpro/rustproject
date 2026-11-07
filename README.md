![Rust Project](docs/banner.png)

Rust Project is a desktop app that keeps track of your side projects. It scans the folders you pick, recognizes the projects inside and puts them on a desk with their next steps, notes and launch buttons for VS Code, a terminal or Claude Code. It is written in Rust with egui.

[Try the web demo](https://djdevpro.github.io/rustproject/)

## Features

- Folder scan up to three levels deep. Parts of a project such as `frontend`, `api` or `db` stay attached to their parent.
- Sorting and one sentence descriptions by a local model through Ollama, or by an AI agent through MCP.
- Fuzzy and semantic search.
- A library with a live graph of domains and projects.
- Updates from GitHub releases, checked at startup.
- An interface in English, French, Spanish, German, Portuguese, Italian, Russian, Chinese, Japanese and Korean. It follows the system language.

![Rust Project web demo showing linked projects in the library](docs/library-demo.webp)

## Install

Download the file for your system from the [releases page](https://github.com/djdevpro/rustproject/releases) and run it. Rust Project updates itself after that.

## Build

```bash
cargo run --release
```

Try it with 30 sample projects. Nothing gets saved.

```bash
cargo run -- --fixture fixtures/demo-en.json
```

## Connect an agent

```bash
claude mcp add rust-project -- "C:\path\to\rust-project.exe" --mcp
```

## Release

Open the Actions tab, run the Release workflow and pick patch, minor or major. The notes come from the commit messages since the last version, unless you type your own. Users see them in the update window.

## License

Rust Project is available under the [MIT license](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option.
