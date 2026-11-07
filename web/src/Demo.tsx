import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  LayoutGrid,
  Layers,
  Settings,
  RefreshCw,
  CircleArrowUp,
  Search,
  Plus,
  Map,
  List,
  Crosshair,
  Waves,
  Archive,
  ArchiveX,
  FlaskConical,
  ArrowRight,
  ChevronDown,
  ChevronUp,
  X,
} from "lucide-react";
import { Button } from "./components/ui/button";
import {
  projects as fixtures,
  colors,
  domainNames,
  type Project,
} from "./data";
import { useI18n } from "./i18n";
import { Graph } from "./Graph";
import { domainIcons } from "./Icons";
import { ProjectPanel, attentionLabels } from "./ProjectPanel";
import { ProjectCard, ProjectList } from "./ProjectCards";
import { Configuration, sections } from "./Configuration";
import {
  SidebarPackages,
  projectPackages,
  type ProjectPackage,
} from "./SidebarPackages";

export function Demo({
  initialPage = "library",
}: { initialPage?: "desk" | "library" } = {}) {
  const { tr, lang } = useI18n();
  const [projects, setProjects] = useState(fixtures);
  const [page, setPage] = useState<string>(initialPage);
  const [section, setSection] = useState("Dossiers sources");
  const [view, setView] = useState("map");
  const [domain, setDomain] = useState<number>();
  const [tourDomain, setTourDomain] = useState<number>();
  const [attention, setAttention] = useState("");
  const [archived, setArchived] = useState(false);
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<number>();
  const [adding, setAdding] = useState(false);
  const [toast, setToast] = useState("");
  const [sort, setSort] = useState("recent");
  const [more, setMore] = useState(false);
  const [allDomains, setAllDomains] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [packageFilter, setPackageFilter] = useState<ProjectPackage>();
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(""), 4000);
    return () => clearTimeout(id);
  }, [toast]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setAdding(false);
        setSelected(undefined);
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  const active = projects.filter((p) => p.status !== "Archived");
  const pool = archived
    ? projects.filter((p) => p.status === "Archived")
    : active.filter((p) => !attention || p.attention === attention);
  const domainOrder = domainNames
    .map((name, index) => ({
      name,
      index,
      count: pool.filter((p) => p.domain === index).length,
    }))
    .sort((a, b) => b.count - a.count || a.index - b.index);
  const { data: hits = [] } = useQuery({
    queryKey: ["fixture-search", search, projects, lang],
    queryFn: () => {
      const normalize = (s: string) =>
        s
          .toLocaleLowerCase()
          .normalize("NFD")
          .replace(/\p{Diacritic}/gu, "");
      const q = normalize(search.trim());
      const exact = projects.filter((p) =>
        normalize(
          [
            p.name,
            p.kind,
            p.description,
            ...p.stack,
            ...projectPackages(p).map((item) => item.name),
            tr(domainNames[p.domain]),
          ].join(" "),
        ).includes(q),
      );
      if (exact.length) return exact;
      return projects.filter((p) => {
        const text = normalize(p.name);
        let index = 0;
        for (const char of q) {
          const found = text.indexOf(char, index);
          if (found < 0) return false;
          index = found + 1;
        }
        return true;
      });
    },
    staleTime: Infinity,
  });
  const graphProjects = useMemo(
    () =>
      projects
        .filter(
          (p) =>
            (archived ? p.status === "Archived" : p.status !== "Archived") &&
            hits.some((h) => h.id === p.id) &&
            (!packageFilter ||
              projectPackages(p).some(
                (item) =>
                  item.manager === packageFilter.manager &&
                  item.name === packageFilter.name,
              )) &&
            (!attention || p.attention === attention),
        )
        .sort((a, b) =>
          sort === "name"
            ? a.name.localeCompare(b.name)
            : sort === "domain"
              ? a.domain - b.domain
              : a.idle_days - b.idle_days,
        ),
    [projects, archived, hits, attention, sort, packageFilter],
  );
  const displayDomain =
    domain ?? (page === "library" && view === "map" ? tourDomain : undefined);
  const visible = useMemo(
    () =>
      graphProjects.filter(
        (p) => displayDomain === undefined || p.domain === displayDomain,
      ),
    [graphProjects, displayDomain],
  );
  const current = projects.find((p) => p.id === selected);
  const update = (patch: Partial<Project>) =>
    setProjects((ps) =>
      ps.map((p) => (p.id === selected ? { ...p, ...patch } : p)),
    );
  const clear = () => {
    setDomain(undefined);
    setAttention("");
    setArchived(false);
    setSearch("");
    setPackageFilter(undefined);
  };
  const configure = (target: string) => {
    setPage("config");
    setSection(target);
    setSelected(undefined);
  };
  const drop = (e: React.DragEvent, patch: Partial<Project>) => {
    e.preventDefault();
    const id = Number(e.dataTransfer.getData("text/project-id"));
    setProjects((ps) => ps.map((p) => (p.id === id ? { ...p, ...patch } : p)));
  };
  const nav = [
    ["desk", "Mon bureau", LayoutGrid],
    ["library", "Bibliothèque", Layers],
    ["config", "Configuration", Settings],
  ] as const;
  const searchbar = (
    <label className="native-search">
      <Search size={20} />
      <input
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        aria-label={tr("Rechercher un projet…")}
        placeholder={tr(
          page === "desk"
            ? "Rechercher un projet…"
            : "Rechercher un nom, un type, une techno (rust, next.js…)",
        )}
      />
      {search && (
        <button aria-label={tr("Tout effacer")} onClick={() => setSearch("")}>
          <X size={16} />
        </button>
      )}
    </label>
  );
  return (
    <section id="demo" className="demo-section">
      <div className="app-scroll">
        <div className="native-window">
          <div className="mac-titlebar">
            <div>
              <i />
              <i />
              <i />
            </div>
            <span>Rust Project</span>
            <span />
          </div>
          <header className="native-topbar">
            <strong className="native-brand">Rust Project</strong>
            <nav aria-label={tr("Le logiciel")}>
              {nav.map(([id, label, Icon]) => (
                <button
                  key={id}
                  disabled={id === "config"}
                  aria-current={page === id ? "page" : undefined}
                  className={page === id ? "selected" : ""}
                  onClick={() => {
                    setPage(id);
                    clear();
                    setSelected(id === "desk" ? 1 : undefined);
                  }}
                >
                  <Icon size={21} />
                  {tr(label)}
                </button>
              ))}
            </nav>
            <div className="scan-status">
              <span className="status-dot" />
              {tr("Dossiers scannés il y a 6 h")}
              <button
                aria-label={tr("Scanner maintenant")}
                onClick={() =>
                  setToast(
                    tr(
                      "Simulation terminée. Les {count} projets fictifs sont disponibles.",
                      { count: projects.length },
                    ),
                  )
                }
              >
                <RefreshCw size={19} />
              </button>
              <button
                aria-label={tr("Mises à jour")}
                onClick={() =>
                  setToast(
                    tr("Aucun service local n'est contacté dans cette démo."),
                  )
                }
              >
                <CircleArrowUp size={20} />
              </button>
            </div>
          </header>
          <div className={`native-body ${current ? "has-inspector" : ""}`}>
            {page === "library" && (
              <aside
                className="native-sidebar"
                onDragEnter={() => setDragging(true)}
                onDragLeave={(event) => {
                  if (
                    !event.currentTarget.contains(
                      event.relatedTarget as Node | null,
                    )
                  )
                    setDragging(false);
                }}
                onDrop={() => setDragging(false)}
              >
                <span className="caps">{tr("Attention")}</span>
                {[
                  ["Focus", "Focus", Crosshair],
                  ["Flow", "Au fil de l'eau", Waves],
                  ["Reserve", "En réserve", Archive],
                ].map(([id, label, Icon]) => {
                  const I = Icon as typeof Crosshair;
                  return (
                    <button
                      key={String(id)}
                      className={
                        attention === id && !archived ? "selected" : ""
                      }
                      onDragOver={(e) => e.preventDefault()}
                      onDrop={(e) => drop(e, { attention: String(id) })}
                      onClick={() => {
                        setArchived(false);
                        setAttention(attention === id ? "" : String(id));
                      }}
                    >
                      <I size={20} />
                      <span>{tr(String(label))}</span>
                      <small>
                        {active.filter((p) => p.attention === id).length}
                      </small>
                    </button>
                  );
                })}
                <button
                  className={archived ? "selected" : ""}
                  onClick={() => {
                    setArchived(!archived);
                    setAttention("");
                  }}
                >
                  <ArchiveX size={20} />
                  <span>{tr("Archivés")}</span>
                  <small>{projects.length - active.length}</small>
                </button>
                <span className="caps domain-caption">{tr("Domaines")}</span>
                <button
                  className={displayDomain === undefined ? "selected" : ""}
                  onClick={() => setDomain(undefined)}
                >
                  <LayoutGrid size={20} />
                  <span>{tr("Tous les domaines")}</span>
                  <small>{pool.length}</small>
                </button>
                {domainOrder
                  .filter(
                    ({ index }, position) =>
                      allDomains ||
                      dragging ||
                      position < 5 ||
                      index === displayDomain,
                  )
                  .map(({ name, index: i, count }) => {
                    const Icon = domainIcons[i];
                    return (
                      <button
                        key={name}
                        className={displayDomain === i ? "selected" : ""}
                        onDragOver={(e) => e.preventDefault()}
                        onDrop={(e) => drop(e, { domain: i })}
                        onClick={() => setDomain(domain === i ? undefined : i)}
                      >
                        <Icon size={20} style={{ color: colors[i] }} />
                        <span>{tr(name)}</span>
                        <small>{count}</small>
                      </button>
                    );
                  })}
                {domainNames.length > 5 && !dragging && (
                  <button
                    className="sidebar-more"
                    onClick={() => setAllDomains((value) => !value)}
                  >
                    {tr(allDomains ? "Réduire" : "Tout voir ({0})", {
                      "0": domainNames.length,
                    })}
                    {allDomains ? (
                      <ChevronUp size={14} />
                    ) : (
                      <ChevronDown size={14} />
                    )}
                  </button>
                )}
                <SidebarPackages
                  projects={pool}
                  selected={packageFilter}
                  onSelect={setPackageFilter}
                />
              </aside>
            )}
            {page === "config" && (
              <aside className="config-sidebar">
                <h2>{tr("Configuration")}</h2>
                {sections.map((s) => (
                  <button
                    key={s}
                    className={section === s ? "selected" : ""}
                    onClick={() => setSection(s)}
                  >
                    {tr(s)}
                  </button>
                ))}
              </aside>
            )}
            <main className={`native-workspace ${page}`}>
              {page === "library" && (
                <>
                  <div className="library-heading">
                    <h2>
                      {tr("Bibliothèque")}
                      <span className="count-pill">{visible.length}</span>
                    </h2>
                    <div className="native-view-switch">
                      {[
                        ["map", "Carte", Map],
                        ["grid", "Grille", LayoutGrid],
                        ["list", "Liste", List],
                      ].map(([id, label, Icon]) => {
                        const I = Icon as typeof Map;
                        return (
                          <button
                            key={String(id)}
                            className={view === id ? "selected" : ""}
                            aria-pressed={view === id}
                            onClick={() => setView(String(id))}
                          >
                            <I size={17} />
                            {tr(String(label))}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                  <div className="library-tools">
                    {searchbar}
                    {(domain !== undefined ||
                      attention ||
                      archived ||
                      packageFilter) && (
                      <button className="filter-chip" onClick={clear}>
                        {packageFilter
                          ? packageFilter.name
                          : tr(
                              domain !== undefined
                                ? domainNames[domain]
                                : archived
                                  ? "Archivés"
                                  : attentionLabels[attention],
                            )}
                        <X size={14} />
                      </button>
                    )}
                    {view !== "map" && (
                      <select
                        aria-label={tr("Trier par")}
                        value={sort}
                        onChange={(e) => setSort(e.target.value)}
                      >
                        <option value="recent">{tr("Activité récente")}</option>
                        <option value="name">{tr("Nom")}</option>
                        <option value="domain">{tr("Domaine")}</option>
                      </select>
                    )}
                  </div>
                  {!(view === "map" ? graphProjects : visible).length ? (
                    <div className="empty-state">
                      <Search size={28} />
                      <p>{tr("Aucun projet ne correspond.")}</p>
                      <Button onClick={clear} variant="outline">
                        {tr("Tout effacer")}
                      </Button>
                    </div>
                  ) : view === "map" ? (
                    <Graph
                      projects={graphProjects}
                      selected={selected}
                      focusedDomain={domain}
                      onTourFocus={setTourDomain}
                      onSelect={(p) => setSelected(p.id)}
                      onDomain={(next) =>
                        setDomain((current) =>
                          current === next ? undefined : next,
                        )
                      }
                    />
                  ) : view === "grid" ? (
                    <div className="native-grid">
                      {visible.map((p) => (
                        <ProjectCard
                          project={p}
                          selected={selected === p.id}
                          onSelect={() => setSelected(p.id)}
                          key={p.id}
                        />
                      ))}
                    </div>
                  ) : (
                    <ProjectList
                      projects={visible}
                      selected={selected}
                      onSelect={(p) => setSelected(p.id)}
                    />
                  )}
                </>
              )}
              {page === "desk" && (
                <div className="desk-scroll">
                  <div className="desk-tools">
                    {searchbar}
                    <Button onClick={() => setAdding(true)}>
                      <Plus size={21} />
                      {tr("Ajouter un projet")}
                    </Button>
                  </div>
                  <div className="desk-heading">
                    <h2>{tr("Mes projets du moment")}</h2>
                    <button
                      className="text-button muted"
                      onClick={() => setProjects((ps) => [...ps].reverse())}
                    >
                      {tr("Réorganiser")}
                    </button>
                  </div>
                  <div className="focus-grid">
                    {active
                      .filter(
                        (p) =>
                          p.attention === "Focus" &&
                          hits.some((h) => h.id === p.id),
                      )
                      .map((p) => (
                        <ProjectCard
                          desk
                          project={p}
                          selected={selected === p.id}
                          onSelect={() => setSelected(p.id)}
                          key={p.id}
                        />
                      ))}
                  </div>
                  <div className="desk-heading">
                    <h2>
                      {tr("Au fil de l'eau")}
                      <span className="count-pill">
                        {active.filter((p) => p.attention === "Flow").length}
                      </span>
                    </h2>
                    <button
                      className="text-button"
                      onClick={() => {
                        setPage("library");
                        setView("list");
                        setAttention("Flow");
                      }}
                    >
                      {tr("Tout voir dans la bibliothèque")}
                      <ArrowRight size={14} />
                    </button>
                  </div>
                  <ProjectList
                    projects={active
                      .filter(
                        (p) =>
                          p.attention === "Flow" &&
                          hits.some((h) => h.id === p.id),
                      )
                      .slice(0, more ? undefined : 6)}
                    selected={selected}
                    onSelect={(p) => setSelected(p.id)}
                  />
                  <button className="show-more" onClick={() => setMore(!more)}>
                    {tr(more ? "Réduire" : "Afficher les autres")}
                  </button>
                  <div className="desk-heading">
                    <h2>{tr("À redécouvrir")}</h2>
                  </div>
                  {active
                    .filter((p) => p.attention === "Reserve")
                    .slice(0, 1)
                    .map((p) => (
                      <div className="rediscover" key={p.id}>
                        <div>
                          <strong>{p.name}</strong>
                          <p>{p.description}</p>
                        </div>
                        <Button
                          variant="outline"
                          onClick={() => {
                            setProjects((ps) =>
                              ps.map((x) =>
                                x.id === p.id
                                  ? { ...x, attention: "Focus" }
                                  : x,
                              ),
                            );
                            setSelected(p.id);
                          }}
                        >
                          {tr("Utiliser ce projet en focus")}
                        </Button>
                      </div>
                    ))}
                </div>
              )}
              {page === "config" && (
                <Configuration
                  section={section}
                  notify={setToast}
                  reset={() => {
                    setProjects(fixtures);
                    setToast(
                      tr(
                        "Simulation terminée. Les {count} projets fictifs sont disponibles.",
                        { count: fixtures.length },
                      ),
                    );
                  }}
                />
              )}
            </main>
            {current && page !== "config" && (
              <ProjectPanel
                key={current.id}
                project={current}
                update={update}
                close={() => setSelected(undefined)}
                notify={setToast}
                configure={() => configure("Lanceurs")}
              />
            )}
          </div>
          <footer className="native-footer">
            <button
              onClick={() => {
                setPage("library");
                clear();
              }}
            >
              {tr("Explorer les {n} projets", { n: projects.length })}
              <ArrowRight size={15} />
            </button>
            <span>
              {tr("{n} au total", { n: projects.length })} (
              {tr("{n} archivés", { n: projects.length - active.length })}) ·{" "}
              {tr("{n} non archivés", { n: active.length })} · Focus{" "}
              {active.filter((p) => p.attention === "Focus").length} ·{" "}
              {tr("Au fil de l'eau")}{" "}
              {active.filter((p) => p.attention === "Flow").length}
            </span>
            <span>
              <FlaskConical size={16} />
              {tr("Données de démonstration")}
            </span>
          </footer>
          {toast && (
            <div className="toast" role="status">
              {toast}
            </div>
          )}
          {adding && (
            <div className="modal-backdrop">
              <form
                className="modal"
                role="dialog"
                aria-modal="true"
                aria-label={tr("Ajouter un projet fictif")}
                onSubmit={(e) => {
                  e.preventDefault();
                  const data = new FormData(e.currentTarget);
                  setProjects((ps) => [
                    ...ps,
                    {
                      ...fixtures[0],
                      id: Math.max(...ps.map((p) => p.id)) + 1,
                      name: String(data.get("name")),
                      description: String(data.get("description")),
                      outcome: "",
                      objective: "",
                      last_point: "",
                      next_action: "",
                      steps: [],
                      notes: [],
                      stack: [],
                      attention: "Reserve",
                      domain: 0,
                      tags: [],
                      packages: [],
                      folders: [],
                      activity: [],
                    },
                  ]);
                  setAdding(false);
                  setPage("library");
                  setView("grid");
                  clear();
                }}
              >
                <h2>{tr("Ajouter un projet fictif")}</h2>
                <label>
                  {tr("Nom du projet")}
                  <input name="name" required autoFocus maxLength={80} />
                </label>
                <label>
                  {tr("Description")}
                  <textarea name="description" maxLength={240} />
                </label>
                <div>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setAdding(false)}
                  >
                    {tr("Annuler")}
                  </Button>
                  <Button type="submit">{tr("Ajouter")}</Button>
                </div>
              </form>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
