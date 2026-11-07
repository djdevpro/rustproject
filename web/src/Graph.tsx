import { useEffect, useRef, useState } from "react";
import { Vector2 } from "three";
import { Plus, Minus, Scan, Wind, Pause, Play } from "lucide-react";
import { createGraphScene, type GraphControls } from "./graph-scene";
import { type Project, domainNames } from "./data";
import { domainIcons } from "./Icons";
import { useI18n } from "./i18n";
export function Graph({
  projects,
  selected,
  focusedDomain,
  onSelect,
  onDomain,
  onTourFocus,
}: {
  projects: Project[];
  selected?: number;
  focusedDomain?: number;
  onSelect: (p: Project) => void;
  onDomain: (i: number) => void;
  onTourFocus?: (domain?: number) => void;
}) {
  const { tr, lang } = useI18n();
  const host = useRef<HTMLDivElement>(null),
    labels = useRef<HTMLDivElement>(null),
    iconLayer = useRef<HTMLDivElement>(null);
  const callbacks = useRef({ onSelect, onDomain, onTourFocus });
  callbacks.current = { onSelect, onDomain, onTourFocus };
  const control = useRef<GraphControls>({
    zoom: 1,
    pan: new Vector2(),
    paused: false,
    shake: 0,
    reset: 0,
    autoFit: true,
  });
  control.current.selected = selected;
  control.current.focusedDomain = focusedDomain;
  const scene = useRef<ReturnType<typeof createGraphScene> | null>(null);
  const current = useRef({
    projects,
    names: domainNames.map((name) => tr(name)),
  });
  current.current = { projects, names: domainNames.map((name) => tr(name)) };
  const [paused, setPaused] = useState(false),
    [failed, setFailed] = useState(false),
    [edgeCount, setEdgeCount] = useState(0);
  useEffect(() => {
    if (!host.current || !labels.current || !iconLayer.current) return;
    try {
      const instance = createGraphScene(
        host.current,
        labels.current,
        Array.from(iconLayer.current.children) as HTMLElement[],
        current.current.projects,
        current.current.names,
        control.current,
        (p) => callbacks.current.onSelect(p),
        (i) => callbacks.current.onDomain(i),
        (i) => callbacks.current.onTourFocus?.(i),
      );
      scene.current = instance;
      setFailed(false);
      return () => {
        instance.destroy();
        scene.current = null;
      };
    } catch {
      setFailed(true);
    }
  }, []);
  useEffect(() => {
    if (scene.current)
      setEdgeCount(scene.current.update(projects, current.current.names));
  }, [projects, lang]);
  return (
    <div className="native-graph">
      <div className="graph-toolbar">
        <span>
          {tr(
            "Glisser pour déplacer · molette pour zoomer · cliquer sur un domaine pour filtrer",
          )}
        </span>
        <div>
          <small>
            {projects.length} {tr("projets")} · {edgeCount} {tr("liens")}
          </small>
          <button
            aria-label="Zoom +"
            onClick={() => {
              control.current.autoFit = false;
              control.current.zoom = Math.min(4, control.current.zoom * 1.25);
            }}
          >
            <Plus size={17} />
          </button>
          <button
            aria-label="Zoom −"
            onClick={() => {
              control.current.autoFit = false;
              control.current.zoom = Math.max(0.2, control.current.zoom * 0.8);
            }}
          >
            <Minus size={17} />
          </button>
          <button
            aria-label={tr("Secouer")}
            onClick={() => control.current.shake++}
          >
            <Wind size={19} />
          </button>
          <button
            aria-label={tr("Recentrer")}
            onClick={() => {
              control.current.autoFit = true;
              control.current.reset++;
            }}
          >
            <Scan size={18} />
          </button>
          <button
            aria-label={tr(paused ? "Reprendre" : "Pause")}
            onClick={() => {
              control.current.paused = !paused;
              setPaused(!paused);
            }}
          >
            {paused ? <Play size={15} /> : <Pause size={15} />}
          </button>
        </div>
      </div>
      <div className="graph-canvas" ref={host} />
      <div className="graph-icons" ref={iconLayer}>
        {domainIcons.map((Icon, i) => (
          <span key={i}>
            <Icon size={24} />
          </span>
        ))}
      </div>
      <div className="graph-labels" ref={labels} />
      {failed && (
        <p className="graph-fallback">
          {tr("WebGL indisponible. Utilisez la grille ou la liste.")}
        </p>
      )}
      <div className="native-graph-legend">
        <span>
          <i className="recent" />
          {tr("actif cette semaine")}
        </span>
        <span>
          <i className="older" />
          {tr("plus ancien")}
        </span>
        <span>
          <i className="focus" />
          {tr("focus")}
        </span>
        <span>
          <i className="parked" />
          {tr("en réserve")}
        </span>
        <span>
          <i className="folder-link" />
          {tr("rangés ensemble")}
        </span>
        <span>
          <i className="stack-link" />
          {tr("technos communes (au survol)")}
        </span>
      </div>
    </div>
  );
}
