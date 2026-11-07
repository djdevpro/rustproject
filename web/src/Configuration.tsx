import { useState } from "react";
import {
  Folder,
  Plus,
  RefreshCw,
  Sparkles,
  Network,
  Check,
} from "lucide-react";
import { useI18n } from "./i18n";
import { domainNames, projects } from "./data";
import { Button } from "./components/ui/button";

export const sections = [
  "Dossiers sources",
  "Découverte",
  "IA & modèles",
  "Ontologie",
  "Lanceurs",
  "États & attention",
  "Apparence",
  "Données",
];
export function Configuration({
  section,
  notify,
  reset,
}: {
  section: string;
  notify: (s: string) => void;
  reset: () => void;
}) {
  const { tr, lang, setLang } = useI18n();
  const [concepts, setConcepts] = useState(
    domainNames.map((name) => ({ name, definition: "", manual: false })),
  );
  const [models, setModels] = useState({
    Ollama: "qwen2.5:3b",
    Ollaya: "decider:2b",
    Embeddings: "qwen3-embedding:4b",
  });
  const [roots, setRoots] = useState(["D:\\Projects", "D:\\Clients"]);
  const [editing, setEditing] = useState<number>();
  const [discovery, setDiscovery] = useState([
    ".git",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "README.md",
  ]);
  const [focusLimit, setFocusLimit] = useState(4);
  const simulation = () => notify(tr("Test simulé réussi"));
  return (
    <div className="config-content">
      <h2>{tr(section)}</h2>
      <p className="config-intro">
        {tr("Les réglages de cette démo sont simulés.")}
      </p>
      {section === "Dossiers sources" && (
        <>
          <section className="settings-card">
            <h3>{tr("Dossiers surveillés")}</h3>
            {roots.map((root, i) => (
              <div className="setting-row" key={i}>
                <Folder size={20} />
                <input
                  aria-label={tr("Dossier")}
                  value={root}
                  onChange={(e) =>
                    setRoots(
                      roots.map((r, j) => (j === i ? e.target.value : r)),
                    )
                  }
                />
                <label>
                  {tr("Profondeur")}
                  <select defaultValue="3">
                    <option>1</option>
                    <option>2</option>
                    <option>3</option>
                  </select>
                </label>
              </div>
            ))}
            <Button
              variant="outline"
              onClick={() => setRoots([...roots, "D:\\Nouveau"])}
            >
              <Plus size={16} />
              {tr("Ajouter un dossier source")}
            </Button>
          </section>
          <section className="settings-card">
            <h3>Scan</h3>
            <p>{tr("Aucun service local n'est contacté dans cette démo.")}</p>
            <Button
              variant="outline"
              onClick={() =>
                notify(
                  tr(
                    "Simulation terminée. Les {count} projets fictifs sont disponibles.",
                    { count: projects.length },
                  ),
                )
              }
            >
              <RefreshCw size={16} />
              {tr("Scanner maintenant")}
            </Button>
          </section>
        </>
      )}
      {section === "Découverte" && (
        <section className="settings-card">
          <h3>{tr("Comment reconnaître un projet")}</h3>
          {[
            ".git",
            "Cargo.toml",
            "package.json",
            "pyproject.toml",
            "README.md",
            "AGENTS.md",
            "CLAUDE.md",
          ].map((marker) => (
            <label className="setting-check" key={marker}>
              <input
                type="checkbox"
                checked={discovery.includes(marker)}
                onChange={() =>
                  setDiscovery(
                    discovery.includes(marker)
                      ? discovery.filter((m) => m !== marker)
                      : [...discovery, marker],
                  )
                }
              />
              <code>{marker}</code>
            </label>
          ))}
        </section>
      )}
      {section === "IA & modèles" && (
        <>
          {Object.entries(models).map(([name, value]) => (
            <section className="settings-card" key={name}>
              <h3>{name}</h3>
              <p>
                {tr(
                  name === "Ollama"
                    ? "Résumés et descriptions"
                    : name === "Ollaya"
                      ? "Décisions par concept"
                      : "Recherche par le sens",
                )}
              </p>
              <div className="setting-row">
                <label>
                  {tr("Modèle")}
                  <select
                    value={value}
                    onChange={(e) =>
                      setModels({ ...models, [name]: e.target.value })
                    }
                  >
                    <option>{value}</option>
                    <option>
                      {name === "Ollaya"
                        ? "decider:4b"
                        : name === "Ollama"
                          ? "qwen2.5:7b"
                          : "nomic-embed-text"}
                    </option>
                  </select>
                </label>
                <Button variant="outline" onClick={simulation}>
                  {tr("Tester et lister")}
                </Button>
              </div>
            </section>
          ))}
          <section className="settings-card">
            <h3>Agent MCP</h3>
            <code>claude mcp add rust-project -- rust-project.exe --mcp</code>
          </section>
        </>
      )}
      {section === "Ontologie" && (
        <>
          <section className="settings-card ontology-overview">
            <div>
              <h3>{tr("Vue d'ensemble")}</h3>
              <p>
                {concepts.length} {tr("Concepts").toLowerCase()} ·{" "}
                {tr("Domaine")}
              </p>
            </div>
            <Button
              variant="outline"
              onClick={() => {
                setConcepts([
                  ...concepts,
                  { name: tr("Nouveau concept"), definition: "", manual: true },
                ]);
                setEditing(concepts.length);
              }}
            >
              <Plus size={16} />
              {tr("Ajouter un concept")}
            </Button>
          </section>
          <section className="settings-card">
            <h3>
              <Network size={19} />
              {tr("Domaine")}
            </h3>
            <div className="concept-table">
              {concepts.map((c, i) => (
                <div key={i}>
                  <button
                    onClick={() => setEditing(editing === i ? undefined : i)}
                  >
                    {tr(c.name)}
                    <span>{c.manual ? tr("Manuel") : tr("Modifier")}</span>
                  </button>
                  {editing === i && (
                    <div className="concept-editor">
                      <label>
                        {tr("Nom")}
                        <input
                          value={c.name}
                          onChange={(e) =>
                            setConcepts(
                              concepts.map((n, j) =>
                                i === j
                                  ? { ...n, name: e.target.value, manual: true }
                                  : n,
                              ),
                            )
                          }
                        />
                      </label>
                      <label>
                        {tr("Définition")}
                        <textarea
                          value={c.definition}
                          onChange={(e) =>
                            setConcepts(
                              concepts.map((n, j) =>
                                i === j
                                  ? {
                                      ...n,
                                      definition: e.target.value,
                                      manual: true,
                                    }
                                  : n,
                              ),
                            )
                          }
                        />
                      </label>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </section>
          <section className="settings-card">
            <h3>{tr("Classement par Ollaya")}</h3>
            <p>{tr("Décisions par concept")} · decider:2b</p>
            <Button
              variant="outline"
              onClick={() => notify(tr("Classement simulé terminé"))}
            >
              <Sparkles size={16} />
              {tr("Classer les projets")}
            </Button>
          </section>
        </>
      )}
      {section === "Lanceurs" && (
        <section className="settings-card">
          <h3>{tr("Ouvrir avec")}</h3>
          {[
            ["Claude Code", "claude"],
            ["VS Code", "code ."],
            ["Terminal", "pwsh"],
            ["Explorer", "explorer ."],
          ].map(([name, command]) => (
            <div className="setting-row" key={name}>
              <strong>{name}</strong>
              <code>{command}</code>
              <Button
                variant="outline"
                onClick={() =>
                  notify(
                    tr(
                      "Simulation. Le logiciel de bureau ouvre ce lanceur dans le dossier du projet.",
                    ),
                  )
                }
              >
                {tr("Tester")}
              </Button>
            </div>
          ))}
        </section>
      )}
      {section === "États & attention" && (
        <section className="settings-card">
          <h3>{tr("Focus")}</h3>
          <label className="setting-row">
            {tr("Limiter les projets en focus")}
            <input
              type="number"
              min="1"
              max="30"
              value={focusLimit}
              onChange={(e) => setFocusLimit(Number(e.target.value))}
            />
          </label>
          <label className="setting-check">
            <input type="checkbox" defaultChecked />
            {tr("En réserve")}
          </label>
        </section>
      )}
      {section === "Apparence" && (
        <section className="settings-card">
          <h3>{tr("Préférences")}</h3>
          <label className="setting-row">
            {tr("Langue")}
            <select
              value={lang}
              onChange={(e) => setLang(e.target.value as "fr" | "en")}
            >
              <option value="fr">Français</option>
              <option value="en">English</option>
            </select>
          </label>
          <div className="setting-row">
            {tr("Accent")}
            <span className="chip focus">
              <Check size={13} />
              {tr("Ambre")}
            </span>
          </div>
        </section>
      )}
      {section === "Données" && (
        <section className="settings-card">
          <h3>{tr("Données de démonstration")}</h3>
          <p>
            {tr(
              "Les modifications restent dans cette session de démonstration.",
            )}
          </p>
          <Button variant="outline" onClick={reset}>
            <RefreshCw size={16} />
            {tr("Rétablir les fixtures")}
          </Button>
        </section>
      )}
    </div>
  );
}
