# Press release

**FOR IMMEDIATE RELEASE**

[City], France, November [day], 2026

## Rust Project: a free, open-source desktop app that finds your forgotten side projects and puts them back on your desk

**Rust Project runs on Windows, macOS and Linux. It spots the projects scattered across a computer and has an AI describe and sort them, either a local model through Ollama or an agent such as Claude Code. Then it lays them out with their next steps. No account, no cloud.**

A folder called "test-v2", another called "new-project-final", a GitHub repo nobody has opened since the summer: code for fun long enough and your projects end up everywhere, and you lose track of where you left each one. Rust Project finds them and shows which one to pick up next. It is a personal project built in France by [First Last].

**Find.** You point the app at the folders to explore, and it walks them up to three levels deep. It recognizes projects by their markers:

- a Git repository;
- a `Cargo.toml` or `package.json` file;
- a README;
- instruction files for AI agents.

Parts of the same project (`frontend`, `api`, `db`…) stay attached to their parent instead of becoming separate entries. With GitHub's command-line tool, repos with no local copy show up too. On the development machine, about 400 folders are scanned in two seconds.

**Organize.** Each project gets a card with a one-sentence description, a domain, its technologies, a goal, steps and notes. "My desk" puts current projects up front, with the next action for each. The library shows everything else as a grid, a list or a live map, a graph where projects orbit their domains. Search forgives typos and understands meaning, so you can find a project by what it does, not only by its name.

**Open.** One click or a keyboard shortcut opens the project folder in VS Code, Cursor, a JetBrains IDE, a terminal or Claude Code. With Claude Code, the new conversation starts pre-filled with the steps left to do.

**AI, your way.** To sort and describe projects, Rust Project relies on one of three options:

- **A model running on your computer through Ollama.** Your projects are then sent to no online service.
- **An AI agent such as Claude Code**, through a built-in MCP server with 26 tools and five ready-made commands.
- **Plain rules, no AI.**

You choose the folders to explore, and the agent cannot add any. Deleting a card never touches your files.

> "[Quote to write in your own words. Suggestion: I had dozens of half-started projects spread across several drives and GitHub, and no idea where I'd left them. I wanted one place that tells me what to pick up next, without sending my folders to the cloud.]"
> — [First Last], creator of Rust Project

**Availability.** Rust Project is available today, for free, for Windows (x64), macOS (Intel and Apple Silicon) and Linux (x64) at https://github.com/djdevpro/rustproject/releases. It updates itself afterwards. The source code, written in Rust, is dual-licensed under MIT or Apache 2.0. The interface comes in ten languages.

**To try it without showing your own folders:** open Configuration › Data and turn on "Show the demo data". Sample projects appear, and nothing is saved.

### About Rust Project

Rust Project is free software built by [First Last], [one sentence: job, city, background]. Its first public version came out on November 6, 2026. It is an independent project, not affiliated with the Rust Foundation or the Rust programming language project.

### Media contact

[First Last] · [email] · [phone, optional]

Source code and downloads: https://github.com/djdevpro/rustproject

Press kit: https://github.com/djdevpro/rustproject/tree/main/docs/presse. High-resolution images and a video demo are available on request.
