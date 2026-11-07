import fixture from "./fixtures.json";
import type { Project } from "./data";

// Additional, deterministic demo workspaces. Every member has its own purpose,
// while sharing a parent folder, technologies and ontology concepts with peers.
// The original native fixture remains untouched.
const workspaces = [
  {
    name: "Fieldworks",
    subject: "field recordings",
    names: [
      "Trailpack",
      "Field Scout",
      "Listening Lab",
      "Field Atlas",
      "Terrain Room",
      "Field Recorder",
      "Trail Journal",
    ],
  },
  {
    name: "Northstar",
    subject: "astronomy",
    names: [
      "Ephemeris CLI",
      "Orbit Guide",
      "Star School",
      "Night Planner",
      "Planetarium",
      "Sky Radio",
      "Observation Log",
    ],
  },
  {
    name: "Ember",
    subject: "cooking",
    names: [
      "Recipe Parser",
      "Pantry Agent",
      "Kitchen Lessons",
      "Meal Calendar",
      "Kitchen Builder",
      "Cooking Voice",
      "Recipe Notebook",
    ],
  },
  {
    name: "Tidal",
    subject: "marine observation",
    names: [
      "Buoy Toolkit",
      "Tide Watch",
      "Ocean Classroom",
      "Coast Dashboard",
      "Reef Sandbox",
      "Hydrophone Studio",
      "Shore Notes",
    ],
  },
  {
    name: "Almanac",
    subject: "gardening",
    names: [
      "Seed Catalog CLI",
      "Garden Companion",
      "Botany Cards",
      "Growing Calendar",
      "Garden Model",
      "Garden Dictation",
      "Season Journal",
    ],
  },
  {
    name: "Mosaic",
    subject: "visual storytelling",
    names: [
      "Storyboard Toolkit",
      "Scene Assistant",
      "Story Workshop",
      "Mosaic Board",
      "Scene Composer",
      "Dialogue Studio",
      "Script Binder",
    ],
  },
  {
    name: "Copper",
    subject: "electronics",
    names: [
      "Circuit Bench",
      "Circuit Reviewer",
      "Electronics Lab",
      "Parts Shelf",
      "Breadboard Sandbox",
      "Signal Listener",
      "Build Notebook",
    ],
  },
  {
    name: "Glasshouse",
    subject: "architecture",
    names: [
      "Plan Converter",
      "Space Assistant",
      "Architecture Study",
      "Material Library",
      "Room Sketcher",
      "Room Acoustics",
      "Design Journal",
    ],
  },
  {
    name: "Footpath",
    subject: "local walking routes",
    names: [
      "GPX Toolkit",
      "Route Assistant",
      "Map Reading Lab",
      "Walk Planner",
      "Route Preview",
      "Walking Audio",
      "Wayfinder Notes",
    ],
  },
  {
    name: "Paperplane",
    subject: "language practice",
    names: [
      "Lexicon Toolkit",
      "Conversation Partner",
      "Phrase School",
      "Practice Calendar",
      "Word Arcade",
      "Pronunciation Studio",
      "Travel Phrasebook",
    ],
  },
  {
    name: "Pulse",
    subject: "music practice",
    names: [
      "MIDI Inspector",
      "Practice Companion",
      "Rhythm School",
      "Session Planner",
      "Rhythm Playground",
      "Loop Studio",
      "Practice Journal",
    ],
  },
  {
    name: "Lantern",
    subject: "local history",
    names: [
      "Archive Importer",
      "Archive Guide",
      "History Lessons",
      "Archive Explorer",
      "Timeline Room",
      "Oral History Studio",
      "Archive Notebook",
    ],
  },
  {
    name: "Kiln",
    subject: "ceramics",
    names: [
      "Glaze Importer",
      "Firing Assistant",
      "Clay Workshop",
      "Kiln Scheduler",
      "Pottery Preview",
      "Workshop Recorder",
      "Glaze Journal",
    ],
  },
  {
    name: "Switchyard",
    subject: "model railways",
    names: [
      "Track Validator",
      "Dispatch Assistant",
      "Railway Lessons",
      "Rolling Stock Shelf",
      "Track Layout Studio",
      "Station Soundboard",
      "Layout Logbook",
    ],
  },
  {
    name: "Origami",
    subject: "paper folding",
    names: [
      "Crease Converter",
      "Fold Companion",
      "Paper School",
      "Pattern Catalog",
      "Fold Simulator",
      "Folding Narrator",
      "Paper Notebook",
    ],
  },
  {
    name: "Weatherline",
    subject: "weather observations",
    names: [
      "Station Data CLI",
      "Forecast Reviewer",
      "Cloud Classroom",
      "Weather Station Board",
      "Atmosphere Sandbox",
      "Weather Bulletin",
      "Cloud Diary",
    ],
  },
  {
    name: "Spindle",
    subject: "textile patterns",
    names: [
      "Pattern Linter",
      "Stitch Assistant",
      "Weaving Lessons",
      "Thread Inventory",
      "Loom Preview",
      "Stitch Counter Voice",
      "Textile Journal",
    ],
  },
  {
    name: "Cairn",
    subject: "geology fieldwork",
    names: ["Sample Importer", "Rock Identifier", "Geology Lab"],
  },
] as const;

const roles = [
  {
    icon: "wrench",
    kind: "Developer tool",
    noun: "tools",
    purpose: "Inspect and convert source files",
    next: "Add a reproducible import example",
    secondary: 3,
    concepts: ["Tool", "Service"],
  },
  {
    icon: "sparkles",
    kind: "AI agent",
    noun: "assistant",
    purpose: "Search the workspace and suggest a next step",
    next: "Check answers against the source notes",
    secondary: 0,
    concepts: ["Tool", "Service"],
  },
  {
    icon: "graduation-cap",
    kind: "Learning app",
    noun: "lessons",
    purpose: "Turn reference material into short practice sessions",
    next: "Review the first exercise sequence",
    secondary: 6,
    concepts: ["App", "Content"],
  },
  {
    icon: "globe",
    kind: "Web app",
    noun: "dashboard",
    purpose: "Organize collections and track ongoing work",
    next: "Connect the list filters to saved views",
    secondary: 1,
    concepts: ["App", "Service"],
  },
  {
    icon: "box",
    kind: "Interactive simulation",
    noun: "sandbox",
    purpose: "Explore the subject through an interactive scene",
    next: "Profile the scene on a small screen",
    secondary: 2,
    concepts: ["App", "Content"],
  },
  {
    icon: "audio-waveform",
    kind: "Audio app",
    noun: "audio",
    purpose: "Record, annotate and replay audio examples",
    next: "Check the input and playback levels",
    secondary: 1,
    concepts: ["App", "Tool"],
  },
  {
    icon: "notebook",
    kind: "Note app",
    noun: "notes",
    purpose: "Keep observations, references and project notes together",
    next: "Add links between related notes",
    secondary: 3,
    concepts: ["App", "Content"],
  },
] as const;

const stacks = [
  ["TypeScript", "React", "Vite"],
  ["Rust", "Tokio", "SQLite"],
  ["Python", "FastAPI", "SQLite"],
  ["PHP", "Laravel", "PostgreSQL"],
  ["Go", "Chi", "PostgreSQL"],
] as const;
const roleTech = [
  "MCP",
  "Ollama",
  "Markdown",
  "HTTP",
  "WebGL",
  "Web Audio",
  "Markdown",
];
const conceptId = new Map(fixture.ontology.concepts.map((c) => [c.name, c.id]));
const baseTime = 1793955600;
const slug = (value: string) => value.toLowerCase().replace(/[^a-z0-9]+/g, "-");

export const generatedProjects: Project[] = workspaces.flatMap(
  (workspace, family) =>
    workspace.names.map((name, domain) => {
      const role = roles[domain];
      const index = family * 7 + domain;
      const idle = [0, 2, 5, 9, 14, 23, 41, 66][(family + domain * 2) % 8];
      const root = `D:\\Projects\\Labs\\${workspace.name}`;
      const folder = `${root}\\${slug(name)}`;
      const kind =
        stacks[family % stacks.length][0] === "Rust" && domain !== 1
          ? "Desktop app"
          : role.kind;
      const status = idle > 40 ? (domain % 2 ? "Idea" : "Paused") : "Active";
      const tags = [
        {
          concept: conceptId.get(fixture.domains[domain].name)!,
          weight: 1,
          manual: false,
        },
        {
          concept: conceptId.get(fixture.domains[role.secondary].name)!,
          weight: 0.64,
          manual: false,
        },
        ...role.concepts.map((name) => ({
          concept: conceptId.get(name)!,
          weight: 0.82,
          manual: false,
        })),
      ];
      return {
        id: 1001 + index,
        name,
        icon: role.icon,
        description: `${role.purpose} for ${workspace.subject}. Part of the ${workspace.name} workspace.`,
        outcome: `${workspace.name} ${role.noun} prototype`,
        objective: `Build a usable ${role.noun} prototype around the shared ${workspace.name} collection.`,
        last_point:
          "The sample collection is imported and the first workflow is connected.",
        next_action: role.next + ".",
        steps: [
          { text: "Import the shared sample collection", done: true },
          { text: role.next, done: false },
          { text: "Document the workspace handoff", done: false },
        ],
        status,
        attention: idle > 30 ? "Reserve" : "Flow",
        domain,
        kind,
        stack: [...stacks[family % stacks.length], roleTech[domain]],
        folders: [folder],
        links: [],
        notes: [
          `Demo workspace. Shares source material with the other ${workspace.name} projects.`,
        ],
        key_files: [
          "README.md",
          stacks[family % stacks.length][0] === "Rust"
            ? "Cargo.toml"
            : stacks[family % stacks.length][0] === "Python"
              ? "pyproject.toml"
              : stacks[family % stacks.length][0] === "PHP"
                ? "composer.json"
                : stacks[family % stacks.length][0] === "Go"
                  ? "go.mod"
                  : "package.json",
        ],
        launchers: [],
        hidden: [],
        favorite: null,
        idle_days: idle,
        review: "Every week",
        confidence: 0.82,
        order: 30 + index,
        activity: [
          { at: baseTime - idle * 86400, kind: "Edit", text: role.next },
          {
            at: baseTime - (idle + 2) * 86400,
            kind: "Scan",
            text: `Found in ${root}`,
          },
        ],
        source: "Local",
        remote: "",
        added_by: "Scan",
        tags,
        tags_hash: 0,
      };
    }),
);
