import { useState, type ReactNode } from "react";
import {
  X,
  Info,
  Crosshair,
  FileText,
  Zap,
  ListChecks,
  Calendar,
  Tag,
  Plus,
  Settings2,
  Sparkles,
  Code,
  Terminal,
  FolderOpen,
  Copy,
} from "lucide-react";
import { type Project, domainNames } from "./data";
import { useI18n } from "./i18n";
import { Button } from "./components/ui/button";

export const statusLabels: Record<string, string> = {
  Active: "En cours",
  Paused: "En pause",
  Idea: "Idée",
  Done: "Terminé",
  Archived: "Archivé",
};
export const attentionLabels: Record<string, string> = {
  Focus: "Focus",
  Flow: "Au fil de l'eau",
  Reserve: "En réserve",
};
export function ProjectPanel({
  project: p,
  update,
  close,
  notify,
  configure,
}: {
  project: Project;
  update: (patch: Partial<Project>) => void;
  close: () => void;
  notify: (text: string) => void;
  configure: () => void;
}) {
  const { tr } = useI18n();
  const [tab, setTab] = useState("Résumé");
  const [newStep, setNewStep] = useState("");
  const [note, setNote] = useState("");
  const field = (
    label: string,
    value: string,
    onChange: (s: string) => void,
    multi = false,
  ) => (
    <label className="edit-field">
      {tr(label)}
      {multi ? (
        <textarea value={value} onChange={(e) => onChange(e.target.value)} />
      ) : (
        <input value={value} onChange={(e) => onChange(e.target.value)} />
      )}
    </label>
  );
  const row = (Icon: typeof Info, title: string, children: ReactNode) => (
    <div className="info-row">
      <Icon size={21} />
      <div>
        <h4>{tr(title)}</h4>
        {children}
      </div>
    </div>
  );
  return (
    <aside className="native-inspector" aria-label={p.name}>
      <div className="panel-heading">
        <div>
          <h2>{p.name}</h2>
          <p>{p.outcome}</p>
          <div className="chips">
            <select
              aria-label={tr("Changer l'état")}
              className={`chip ${p.status === "Active" ? "good" : ""}`}
              value={p.status}
              onChange={(e) => update({ status: e.target.value })}
            >
              {Object.entries(statusLabels).map(([value, label]) => (
                <option value={value} key={value}>
                  {tr(label)}
                </option>
              ))}
            </select>
            <select
              aria-label={tr("Changer l'attention")}
              className={`chip ${p.attention === "Focus" ? "focus" : ""}`}
              value={p.attention}
              onChange={(e) => update({ attention: e.target.value })}
            >
              {Object.entries(attentionLabels).map(([value, label]) => (
                <option value={value} key={value}>
                  {tr(label)}
                </option>
              ))}
            </select>
          </div>
        </div>
        <button
          className="icon-button close-panel"
          aria-label={tr("Fermer le panneau")}
          onClick={close}
        >
          <X size={19} />
        </button>
      </div>
      <div className="launcher-heading">
        <span className="caps">{tr("Ouvrir avec")}</span>
        <button className="text-button" disabled onClick={configure}>
          {tr("Gérer")}
          <Settings2 size={13} />
        </button>
      </div>
      <div className="native-launchers">
        {[
          [Sparkles, "Claude Code"],
          [Code, "VS Code"],
          [Terminal, tr("Terminal")],
          [FolderOpen, tr("Explorateur")],
        ].map(([Icon, label], i) => {
          const I = Icon as typeof Code;
          return (
            <button
              key={String(label)}
              className={i === 0 ? "featured" : ""}
              onClick={() =>
                notify(
                  tr(
                    "Simulation. Le logiciel de bureau ouvre ce lanceur dans le dossier du projet.",
                  ),
                )
              }
            >
              <span>
                <I size={19} />
              </span>
              <small>{String(label)}</small>
            </button>
          );
        })}
        <button className="add-launcher" disabled onClick={configure}>
          <Plus size={23} />
          <small>{tr("Ajouter")}</small>
        </button>
      </div>
      <div className="panel-tabs" role="tablist">
        {["Résumé", "Activité", "Ressources", "Modifier"].map((name) => (
          <button
            key={name}
            role="tab"
            disabled={name === "Modifier"}
            aria-selected={tab === name}
            onClick={() => setTab(name)}
          >
            {tr(name)}
          </button>
        ))}
      </div>
      <div className="panel-content">
        {tab === "Résumé" && (
          <>
            {row(Info, "En une phrase", <p>{p.description}</p>)}
            {row(Crosshair, "Objectif", <p>{p.objective}</p>)}
            {row(FileText, "Dernier point", <p>{p.last_point}</p>)}
            {row(
              Zap,
              "Prochaine action",
              <p className="strong">{p.next_action}</p>,
            )}
            {row(
              ListChecks,
              "Prochaines étapes",
              <>
                <span className="steps-count">
                  {p.steps.filter((s) => s.done).length}/{p.steps.length}
                </span>
                {p.steps.map((s, i) => (
                  <div className="step-line" key={i}>
                    <label>
                      <input
                        type="checkbox"
                        checked={s.done}
                        onChange={() =>
                          update({
                            steps: p.steps.map((step, j) =>
                              j === i ? { ...step, done: !step.done } : step,
                            ),
                          })
                        }
                      />
                      <span className={s.done ? "done" : ""}>{s.text}</span>
                    </label>
                    <button
                      aria-label={tr("Supprimer cette étape")}
                      onClick={() =>
                        update({ steps: p.steps.filter((_, j) => j !== i) })
                      }
                    >
                      <X size={13} />
                    </button>
                  </div>
                ))}
                <form
                  className="add-step"
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (newStep.trim()) {
                      update({
                        steps: [
                          ...p.steps,
                          { text: newStep.trim(), done: false },
                        ],
                      });
                      setNewStep("");
                    }
                  }}
                >
                  <input
                    placeholder={tr("Ajouter une étape…")}
                    aria-label={tr("Ajouter une étape…")}
                    value={newStep}
                    onChange={(e) => setNewStep(e.target.value)}
                  />
                  <button aria-label={tr("Ajouter l'étape")}>
                    <Plus size={17} />
                  </button>
                </form>
              </>,
            )}
            {row(Calendar, "Prochaine revue", <p>{p.review}</p>)}
            {row(
              Tag,
              "Tags",
              <div className="chips">
                <span className="chip">{tr(domainNames[p.domain])}</span>
                {p.stack.map((s) => (
                  <span key={s} className="chip">
                    {s}
                  </span>
                ))}
              </div>,
            )}
          </>
        )}
        {tab === "Activité" && (
          <>
            <form
              className="note-form"
              onSubmit={(e) => {
                e.preventDefault();
                if (note.trim()) {
                  update({ notes: [...p.notes, note.trim()] });
                  setNote("");
                }
              }}
            >
              <input
                placeholder={tr("Noter un point d'étape…")}
                aria-label={tr("Noter un point d'étape…")}
                value={note}
                onChange={(e) => setNote(e.target.value)}
              />
              <Button type="submit" variant="outline">
                {tr("Noter")}
              </Button>
            </form>
            {[...p.notes].reverse().map((n, i) => (
              <div className="activity-row" key={i}>
                <FileText size={18} />
                <p>{n}</p>
              </div>
            ))}
            {p.activity.map((a, i) => (
              <div className="activity-row" key={"a" + i}>
                <span className="activity-dot" />
                <p>{a.text}</p>
              </div>
            ))}
          </>
        )}
        {tab === "Ressources" && (
          <>
            <h4>{tr("Dossiers")}</h4>
            {p.folders.map((folder) => (
              <div className="resource-row" key={folder}>
                <FolderOpen size={17} />
                <code>{folder}</code>
                <button
                  aria-label={tr("Copier")}
                  onClick={() =>
                    navigator.clipboard
                      .writeText(folder)
                      .then(() => notify(tr("Copié")))
                      .catch(() => notify(folder))
                  }
                >
                  <Copy size={15} />
                </button>
              </div>
            ))}
            <h4>{tr("Fichiers clés · relevés au scan")}</h4>
            <div className="chips">
              {p.key_files.map((f) => (
                <code className="chip" key={f}>
                  {f}
                </code>
              ))}
            </div>
            <h4>{tr("Notes")}</h4>
            {p.notes.map((n, i) => (
              <p className="resource-note" key={i}>
                {n}
              </p>
            ))}
          </>
        )}
        {tab === "Modifier" && (
          <>
            <section className="edit-section">
              <h3>{tr("Fiche")}</h3>
              {field("Nom du projet", p.name, (name) => update({ name }))}
              {field(
                "Description",
                p.description,
                (description) => update({ description }),
                true,
              )}
              {field("Résultat recherché", p.outcome, (outcome) =>
                update({ outcome }),
              )}
            </section>
            <section className="edit-section">
              <h3>{tr("Suivi")}</h3>
              {field("Prochaine action", p.next_action, (next_action) =>
                update({ next_action }),
              )}
              {field(
                "Objectif",
                p.objective,
                (objective) => update({ objective }),
                true,
              )}
              {field(
                "Dernier point",
                p.last_point,
                (last_point) => update({ last_point }),
                true,
              )}
            </section>
            <section className="edit-section">
              <h3>{tr("Classement")}</h3>
              <label className="edit-field">
                {tr("Domaine")}
                <select
                  value={p.domain}
                  onChange={(e) => update({ domain: Number(e.target.value) })}
                >
                  {domainNames.map((n, i) => (
                    <option key={n} value={i}>
                      {tr(n)}
                    </option>
                  ))}
                </select>
              </label>
            </section>
            <p className="save-hint">
              {tr(
                "Les modifications restent dans cette session de démonstration.",
              )}
            </p>
          </>
        )}
      </div>
    </aside>
  );
}
