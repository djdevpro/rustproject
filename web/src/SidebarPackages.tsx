import { useId, useMemo, useState } from "react";
import {
  ChevronDown,
  ChevronUp,
  Container,
  Network,
  Package,
} from "lucide-react";
import { type Project, domainNames } from "./data";
import fixture from "./fixtures.json";
import { useI18n } from "./i18n";

export type ProjectPackage = { manager: string; name: string };
export const projectPackages = (project: Project): ProjectPackage[] =>
  (project as Project & { packages?: ProjectPackage[] }).packages ?? [];

// Order and colors from src/packages.rs, Manager::ALL / Manager::color.
const managers = [
  ["Npm", "npm", "#e2574c"],
  ["Pip", "pip", "#5b9bd5"],
  ["Cargo", "Cargo", "#dea584"],
  ["Composer", "Composer", "#b8916b"],
  ["Go", "Go", "#4fc3d9"],
  ["Maven", "Maven", "#e07a5f"],
  ["Gradle", "Gradle", "#5fb3a1"],
  ["NuGet", "NuGet", "#8f7ae0"],
  ["RubyGems", "RubyGems", "#d9534f"],
  ["Pub", "pub", "#42a5f5"],
  ["SwiftPm", "Swift", "#f28c4a"],
  ["CocoaPods", "CocoaPods", "#e8a15a"],
  ["Hex", "Hex", "#a98bd6"],
  ["Deno", "Deno", "#9aa5b1"],
  ["Conda", "conda", "#7cb86f"],
  ["Vcpkg", "vcpkg", "#6b8fcf"],
  ["Conan", "Conan", "#6fb1d6"],
  ["Julia", "Julia", "#b07cc9"],
  ["Cran", "CRAN", "#6a9fd8"],
] as const;

export function SidebarPackages({
  projects,
  selected,
  onSelect,
}: {
  projects: Project[];
  selected?: ProjectPackage;
  onSelect: (selected?: ProjectPackage) => void;
}) {
  const { tr } = useI18n();
  const id = useId();
  const [open, setOpen] = useState<string[]>([]);
  const [all, setAll] = useState<string[]>([]);
  const [taxonomyOpen, setTaxonomyOpen] = useState(false);
  const counts = useMemo(() => {
    const counts = new Map<string, Map<string, number>>();
    for (const project of projects) {
      const distinct = new Set<string>();
      for (const item of projectPackages(project)) {
        const key = `${item.manager}\0${item.name}`;
        if (distinct.has(key)) continue;
        distinct.add(key);
        if (!counts.has(item.manager)) counts.set(item.manager, new Map());
        const names = counts.get(item.manager)!;
        names.set(item.name, (names.get(item.name) ?? 0) + 1);
      }
    }
    return new Map(
      [...counts].map(([manager, names]) => [
        manager,
        [...names].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])),
      ]),
    );
  }, [projects]);
  const toggle = (items: string[], manager: string) =>
    items.includes(manager)
      ? items.filter((value) => value !== manager)
      : [...items, manager];
  const rows = (manager: string, color: string) => {
    const entries = counts.get(manager) ?? [];
    const expanded = all.includes(manager);
    const Icon = manager === "Docker" ? Container : Package;
    return (
      <>
        {entries
          .filter(
            ([name], index) =>
              expanded ||
              index < 5 ||
              (selected?.manager === manager && selected.name === name),
          )
          .map(([name, count]) => {
            const active =
              selected?.manager === manager && selected.name === name;
            return (
              <button
                key={name}
                className={`package-entry ${active ? "selected" : ""}`}
                aria-pressed={active}
                onClick={() => onSelect(active ? undefined : { manager, name })}
              >
                <Icon size={17} style={{ color }} />
                <span>{name}</span>
                <small>{count}</small>
              </button>
            );
          })}
        {entries.length > 5 && (
          <button
            className="sidebar-more"
            onClick={() => setAll((items) => toggle(items, manager))}
          >
            {tr(expanded ? "Réduire" : "Tout voir ({0})", {
              "0": entries.length,
            })}
            {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>
        )}
      </>
    );
  };
  return (
    <div className="sidebar-packages">
      <span className="caps">{tr("Paquets")}</span>
      {managers
        .filter(([manager]) => counts.has(manager))
        .map(([manager, name, color]) => {
          const expanded =
            open.includes(manager) || selected?.manager === manager;
          const total = counts.get(manager)!.length;
          return (
            <div className="package-manager" key={manager}>
              <button
                className="package-manager-heading"
                aria-expanded={expanded}
                aria-controls={`${id}-${manager}`}
                onClick={() => setOpen((items) => toggle(items, manager))}
              >
                <i style={{ background: color }} />
                <span>{name}</span>
                <small>
                  {tr(total === 1 ? "1 paquet" : "{0} paquets", { "0": total })}
                </small>
                {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
              </button>
              {expanded && (
                <div id={`${id}-${manager}`}>{rows(manager, color)}</div>
              )}
            </div>
          );
        })}
      {!!counts.get("Docker")?.length && (
        <section className="docker-packages">
          <span className="caps">{tr("Images Docker")}</span>
          {rows("Docker", "#2496ed")}
        </section>
      )}
      <button
        className="sidebar-taxonomy"
        aria-expanded={taxonomyOpen}
        onClick={() => setTaxonomyOpen((value) => !value)}
      >
        <Network size={16} />
        <span>{tr("Taxonomie")}</span>
        {taxonomyOpen ? <ChevronUp size={15} /> : <ChevronDown size={15} />}
      </button>
      {taxonomyOpen && (
        <div className="taxonomy-summary">
          <p>
            {tr("{0} domaines · {1} tags dans l'ontologie", {
              "0": domainNames.length,
              "1": fixture.ontology.concepts.length,
            })}
          </p>
          <button disabled>{tr("Ontologie")}</button>
        </div>
      )}
    </div>
  );
}
