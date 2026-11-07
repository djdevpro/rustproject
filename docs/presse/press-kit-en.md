# Rust Project press kit

*Updated November 7, 2026 · version 0.1.9*

> Rust Project is an independent project, not affiliated with the Rust Foundation or the Rust programming language project.

## At a glance

**In one sentence.** Rust Project is a free, open-source desktop app that finds the side projects scattered across your computer. An AI, local or agent, sorts them. The app then lays them out on a desk, with their next steps and buttons to reopen them in VS Code, a terminal or Claude Code.

**In thirty seconds.** Anyone who codes for fun ends up with dozens of half-started projects spread across vaguely named folders and GitHub. Rust Project scans the folders you point it at and recognizes the projects. An AI describes each one in a sentence and sorts it by domain. The app then shows them on a desk: current projects at the top, the next action for each, and one click to reopen the folder in your editor. The AI can run entirely on your machine with Ollama, or go through an agent such as Claude Code using the MCP protocol. There is no account and no cloud. The app speaks ten languages and runs on Windows, macOS and Linux.

## Fact sheet

| | |
|---|---|
| Name | Rust Project (previous working name: Atelier) |
| Category | Desktop app for managing personal coding projects |
| Platforms | Windows (x64), macOS (universal binary for Intel and Apple Silicon), Linux (x64) |
| Price | Free |
| License | Open source, MIT or Apache 2.0 at your option |
| Technology | Rust (2024 edition), egui 0.37 interface |
| Languages | 10: English, French, Spanish, German, Portuguese, Italian, Russian, Chinese, Japanese, Korean. The interface follows the system language |
| AI | Optional: a local model via Ollama, an AI agent via MCP (Claude Code…), or rules with no AI |
| Data | One JSON file on your computer, in the `.rust-project` folder of your user account. No account, no server |
| Version | 0.1.9, November 7, 2026. First public release (0.1.0) on November 6, 2026 |
| Download size | 26 MB (Windows), 21 MB (macOS, zip), 42 MB (Linux) |
| Source code | https://github.com/djdevpro/rustproject |
| Downloads | https://github.com/djdevpro/rustproject/releases |
| Creator | [First Last], [city], France |

## The problem

Developers, students and tinkerers pile up projects: an idea started over a weekend, a prototype, a course followed halfway, a home-made tool. These projects sleep in loosely named folders, on several drives or only on GitHub. You forget what they did, where you stopped and what was left to do.

Task managers don't solve this: you would have to type everything in by hand, then keep it up to date.

## What Rust Project does

### Find

- **Source folders.** The app explores the folders you choose, as many as you like, each one to a depth of one to three levels. It skips hidden folders and exclusions such as `node_modules` or `target`.
- **Markers.** It recognizes a project by a `.git` repository, a manifest (`Cargo.toml`, `package.json`…), a `README.md` or instruction files for AI agents (`CLAUDE.md`, `AGENTS.md`).
- **Parts.** `frontend`, `api`, `db`, `site` or `packages/x` folders are attached to their parent project instead of becoming separate cards.
- **GitHub.** It picks up your repositories through the `gh` tool: your own, your organizations' and your collaborations. A repository with no local copy becomes a "GitHub" card.
- **Ignored folders.** One click ignores a copy or a backup, and the scan skips it from then on.
- **Speed.** About 400 folders are scanned in two seconds on the development machine.

### Organize

- **One card per project**, with a one-sentence description, a domain, a type, technologies, a goal, the last checkpoint, the next action, steps to tick off, notes, resources, tags and a logo.
- **Statuses**: Focus, Ongoing, Parked, Archived.
- **My desk** puts current projects first, with their progress and their next action.
- **The library** shows every project as a map, a grid or a list, with a filter by domain.
- **The map** is a living graph:
  - domains are glowing hubs, and projects orbit around them;
  - on hover, particles run along the links;
  - clicking a domain filters it, and the camera glides to it;
  - the mouse wheel zooms;
  - dragging a project onto a status files it.
- **The ontology** organizes everything into facets (domain, nature, purpose…) and concepts. Each concept has a definition, synonyms and a parent. The AI can suggest an ontology, which is applied only with your approval. Concepts you edit by hand are kept.
- **Search** combines three methods:
  - fuzzy, so typos and accents are forgiven;
  - semantic, through embeddings, so a project turns up for words close in meaning;
  - by ontology, so a search leads to a concept and then to the projects attached to it.

### Open

- **Launchers.** Tiles open the project folder in Claude Code, VS Code, Cursor, a JetBrains IDE, a terminal or the file manager (Finder on a Mac). It takes one click or a keyboard shortcut, and you can add any command.
- **Claude Code.** The launcher opens a new conversation in Claude Desktop, on the project folder, pre-filled with the steps left to do.

## AI, your way

| Mode | How it works | Where the data goes |
|---|---|---|
| **Ollama (local AI)** | Each folder is summarized and sent to the chosen model, which answers in JSON constrained by a schema. Recommended models: `qwen2.5:3b` (≈ 1.9 GB) for sorting, `qwen3-embedding:4b` for semantic search | Nowhere, as long as Ollama runs on your machine |
| **AI agent via MCP** | Claude Code, or any MCP client, launches `rust-project --mcp`. The server exposes 26 tools (scan, create and merge cards, tick off a step…) and five ready-made commands, including `ranger_mes_projets` (sort my projects) and `nettoyer_mes_fiches` (clean up my cards) | To the service of the chosen agent, for example Anthropic for Claude Code |
| **No AI** | A folder that carries a marker becomes a project. Its description comes from its manifest or from the first sentence of its README | Nowhere |

**Safeguards**

- You choose the folders to explore. The AI agent cannot add any.
- An ontology suggested by the AI waits for your approval.
- Deleting a card never deletes the folder.
- What the agent writes shows up live in the interface.

## Under the hood (for technical media)

- **Rust and egui.** One native executable per platform, with no embedded browser.
- **Close to 20,000 lines of Rust.** The force simulation behind the map is written by hand, in the spirit of d3-force.
- **Background jobs.** Scanning, sorting, model downloads and embeddings never freeze the interface.
- **Built-in MCP server.** It speaks JSON-RPC over standard input and output. During the handshake, it sends the agent its rules: one card per project, never one for a part, a one-sentence description.
- **Embeddings.** They are requested with a 2,048-token context, so `qwen3-embedding:4b` uses about 4 GB of graphics memory instead of about 10 GB.
- **Data.** A JSON file written atomically (temporary file, then rename), which the interface re-reads every second.
- **Updates.** They come from GitHub releases, are checked at startup and verified with SHA-256.
- **Automated UI tests.** They use egui_kittest and run without a GPU. They never touch real data and never launch a program.
- **Continuous integration.** GitHub Actions builds the Windows, macOS (universal) and Linux binaries for every release.

## Key figures

| Figure | What it measures |
|---|---|
| $0 | Price: no subscription, no account |
| 3 | Platforms: Windows, macOS, Linux |
| 10 | Interface languages |
| ≈ 20,000 | Lines of Rust |
| 26 | MCP tools offered to AI agents |
| ≈ 2 s | Time to scan about 400 folders |
| 1 | Sentence to describe each project |
| 10 | Releases published on November 6 and 7, 2026 (0.1.0 to 0.1.9) |

## Images

Images may be used freely to illustrate an article about Rust Project. Credit: "Rust Project".

| File | Content | Size |
|---|---|---|
| [`docs/banner.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/banner.png) | Banner: logo, tagline "All your side projects in one place." and a screenshot of the desk | 2000 × 667 |
| [`docs/bureau.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/bureau.png) | "My desk": current projects, ongoing projects, an open card with its launchers | 1480 × 920 |
| [`docs/carte.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/carte.png) | Library, map view: domains suggested by the AI, projects in orbit | 1480 × 920 |
| [`docs/grille.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/grille.png) | Library, grid view, with an open card | 1480 × 920 |
| [`assets/icon.svg`](https://raw.githubusercontent.com/djdevpro/rustproject/main/assets/icon.svg), [`assets/icon.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/assets/icon.png) | Logo, as a vector and at 256 × 256 | SVG, PNG |

The screenshots show sample projects, with the interface in English.

## Try it in two minutes

1. **Download** the file for your system from https://github.com/djdevpro/rustproject/releases:
   - Windows: `rust-project-0.1.9-windows-x64.exe`
   - macOS: `rust-project-0.1.9-macos-universal.zip`, which contains the app
   - Linux: `rust-project-0.1.9-linux-x64`, made executable with `chmod +x`
2. **Get past the system warning** on first launch. The binaries are not signed with a publisher certificate.
   - Windows: SmartScreen shows "Windows protected your PC". Click "More info", then "Run anyway".
   - macOS: the first launch is blocked. Open System Settings › Privacy & Security, then click "Open Anyway".
3. **Try it without your own folders.** Open Configuration › Data, then turn on "Show the demo data".
4. **Go further.** Install Ollama and pick a model in Configuration › AI. You can also connect Claude Code:

   ```bash
   claude mcp add rust-project -- "<path to the executable>" --mcp
   ```

## Project status

- **Version 0.1.** The project is young and moves fast. Feedback goes through GitHub issues.
- **Unsigned binaries.** Hence the warnings described above.
- **Optional requirements.** Semantic search needs an embeddings model in Ollama. Importing GitHub repositories needs the `gh` tool.

## About the creator

[Three or four sentences to write: who you are, what you do, why and how you built Rust Project. Journalists often reuse this passage as is. Also have a portrait photo ready: it is often requested.]

## FAQ

**Is it related to the Rust language or the Rust Foundation?**
No. Rust Project is written in Rust, but it is an independent project, neither affiliated with nor endorsed by the Rust Foundation.

**Is it really free?**
Yes. There are no ads and no account, and the code is open source (MIT or Apache 2.0). [Business model, if any, to clarify, or "none: it is a personal project".]

**Do my folders go online?**
Cards stay in a file on your computer. With Ollama installed locally, nothing is sent anywhere. With an AI agent such as Claude Code, what the agent reads goes through its service. The app itself only connects to the internet in three cases:

- to check for updates on GitHub;
- to download an Ollama model, if you ask it to;
- to list your repositories through `gh`.

**Do I need a GPU?**
No. Without AI, the app works with simple rules. With Ollama, the model recommended for sorting weighs about 1.9 GB. The semantic search model needs about 4 GB of graphics memory.

**How is it different from Notion, Trello or GitHub Projects?**
Those tools make you type everything in and keep it up to date by hand. Rust Project starts from your disk: it finds the projects itself, describes them and reopens them in your developer tools.

**Why Rust?**
To get a native, fast app shipped as a single executable per platform, with no embedded browser. [Add your own reason, for example wanting to learn the language.]

**What is MCP?**
The Model Context Protocol is an open protocol that lets AI agents use external tools. Rust Project exposes 26 of them, so an agent such as Claude Code can organize your projects for you.

**What's next?**
[Roadmap to clarify: signed binaries, new integrations, etc.]

## Contact

[First Last] · [email] · [phone, optional] · [LinkedIn or Bluesky profile]

https://github.com/djdevpro/rustproject
