import { ArrowRight, ChevronRight, Zap } from "lucide-react";
import { type Project, colors } from "./data";
import { ProjectIcon } from "./Icons";
import { useI18n } from "./i18n";
import { statusLabels, attentionLabels } from "./ProjectPanel";
export function useAge() {
  const { tr } = useI18n();
  return (days: number) =>
    days === 0
      ? tr("aujourd'hui")
      : days === 1
        ? tr("hier")
        : tr("il y a {n} j", { n: days });
}
export function ProjectCard({
  project: p,
  selected,
  onSelect,
  desk = false,
}: {
  project: Project;
  selected: boolean;
  onSelect: () => void;
  desk?: boolean;
}) {
  const { tr } = useI18n();
  const age = useAge();
  const done = p.steps.filter((s) => s.done).length;
  return (
    <button
      className={`native-card ${selected ? "selected" : ""} ${desk ? "desk-card" : ""}`}
      onClick={onSelect}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("text/project-id", String(p.id));
        e.dataTransfer.effectAllowed = "move";
      }}
    >
      <div className="card-name">
        <span
          className="card-icon"
          style={{ color: desk ? "var(--accent-ink)" : colors[p.domain] }}
        >
          <ProjectIcon name={p.icon} />
        </span>
        <div>
          <h3>{p.name}</h3>
          {!desk && <p>{p.kind}</p>}
        </div>
      </div>
      {desk && (
        <p className="card-outcome">
          {tr("Viser")} {p.outcome}
        </p>
      )}
      <div className="chips">
        <span className={`chip ${p.status === "Active" ? "good" : ""}`}>
          {tr(statusLabels[p.status])}
        </span>
        {!desk && (
          <span className={`chip ${p.attention === "Focus" ? "focus" : ""}`}>
            {tr(attentionLabels[p.attention])}
          </span>
        )}
        {desk && p.idle_days > 0 && <small>{age(p.idle_days)}</small>}
      </div>
      {desk ? (
        <>
          <div className="card-progress">
            <div>
              <i
                style={{
                  width: `${p.steps.length ? (done / p.steps.length) * 100 : 0}%`,
                }}
              />
            </div>
            <span>
              {done}/{p.steps.length}
            </span>
          </div>
          <p className="card-next">
            <Zap size={14} />
            {p.next_action}
          </p>
          <span className="card-details">
            <ArrowRight size={16} />
            {tr("Détails")}
          </span>
        </>
      ) : (
        <>
          <code className="card-stack">{p.stack.join(" · ")}</code>
          <div className="card-bottom">
            <span>{age(p.idle_days)}</span>
            <ChevronRight size={16} />
          </div>
        </>
      )}
    </button>
  );
}
export function ProjectList({
  projects,
  selected,
  onSelect,
}: {
  projects: Project[];
  selected?: number;
  onSelect: (p: Project) => void;
}) {
  const { tr } = useI18n();
  const age = useAge();
  return (
    <div className="native-list">
      {projects.map((p) => (
        <button
          key={p.id}
          className={`native-list-row ${selected === p.id ? "selected" : ""}`}
          onClick={() => onSelect(p)}
        >
          <ProjectIcon name={p.icon} size={20} />
          <strong>{p.name}</strong>
          <div className="chips">
            <span className={`chip ${p.status === "Active" ? "good" : ""}`}>
              {tr(statusLabels[p.status])}
            </span>
            <span className="chip">{tr(attentionLabels[p.attention])}</span>
          </div>
          <p>
            <b>{tr("Prochaine action")}</b> {p.next_action}
          </p>
          <small>{age(p.idle_days)}</small>
          <span className="list-details">
            <ArrowRight size={13} />
            {tr("Détails")}
          </span>
        </button>
      ))}
    </div>
  );
}
