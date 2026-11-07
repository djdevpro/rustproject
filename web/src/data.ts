import fixture from "./fixtures.json";
import { generatedProjects } from "./generated-projects";
export type PackageManager =
  | "Cargo"
  | "Npm"
  | "Composer"
  | "Pip"
  | "Go"
  | "Maven"
  | "Gradle"
  | "NuGet"
  | "RubyGems"
  | "Pub"
  | "SwiftPm"
  | "CocoaPods"
  | "Hex"
  | "Deno"
  | "Conda"
  | "Vcpkg"
  | "Conan"
  | "Julia"
  | "Cran"
  | "Docker";
export type ProjectPackage = { manager: PackageManager; name: string };
export type Project = (typeof fixture.projects)[number] & {
  packages?: ProjectPackage[];
};

// Simulated direct manifest dependencies, using the same manager names as Rust.
// They enrich the web demo without changing its original native fixture file.
const packageProfiles: Record<string, ProjectPackage[]> = {
  Rust: [{ manager: "Cargo", name: "serde" }],
  egui: [{ manager: "Cargo", name: "egui" }],
  Tokio: [{ manager: "Cargo", name: "tokio" }],
  TypeScript: [{ manager: "Npm", name: "typescript" }],
  React: [{ manager: "Npm", name: "react" }],
  Vite: [{ manager: "Npm", name: "vite" }],
  "Next.js": [{ manager: "Npm", name: "next" }],
  "Three.js": [{ manager: "Npm", name: "three" }],
  WebGL: [{ manager: "Npm", name: "three" }],
  "Web Audio": [{ manager: "Npm", name: "tone" }],
  Python: [{ manager: "Pip", name: "pydantic" }],
  FastAPI: [
    { manager: "Pip", name: "fastapi" },
    { manager: "Pip", name: "uvicorn" },
  ],
  PHP: [{ manager: "Composer", name: "guzzlehttp/guzzle" }],
  Laravel: [{ manager: "Composer", name: "laravel/framework" }],
  Go: [{ manager: "Go", name: "github.com/spf13/cobra" }],
  Chi: [{ manager: "Go", name: "github.com/go-chi/chi/v5" }],
};
function demoPackages(project: Project): ProjectPackage[] {
  const packages = project.stack.flatMap((tech) => packageProfiles[tech] ?? []);
  const backend =
    ["API", "AI agent", "MCP server", "SaaS"].includes(project.kind) ||
    project.stack.some((tech) => ["FastAPI", "PHP", "Go"].includes(tech));
  if (backend) {
    packages.push({
      manager: "Docker",
      name: project.stack.includes("Python")
        ? "python"
        : project.stack.includes("Go")
          ? "golang"
          : project.stack.includes("PHP")
            ? "php"
            : "node",
    });
    if (project.stack.includes("PostgreSQL"))
      packages.push({ manager: "Docker", name: "postgres" });
    if (project.domain === 1)
      packages.push({ manager: "Docker", name: "ollama/ollama" });
  }
  return [
    ...new Map(
      packages.map((pkg) => [`${pkg.manager}:${pkg.name}`, pkg]),
    ).values(),
  ].sort(
    (a, b) =>
      a.manager.localeCompare(b.manager) || a.name.localeCompare(b.name),
  );
}
export const projects: Project[] = [
  ...fixture.projects,
  ...generatedProjects,
].map((project) => ({ ...project, packages: demoPackages(project) }));
export const domains = fixture.domains;
export const colors = domains.map(
  (d) => `rgb(${d.color.slice(0, 3).join(",")})`,
);
export const domainNames = [
  "Outils développeur",
  "IA & agents",
  "Apprentissage",
  "Web & SaaS",
  "Jeux & 3D",
  "Audio & voix",
  "Écriture & personnel",
];
export const repository = "https://github.com/djdevpro/rustproject";
