import * as THREE from "three";
import { colors, type Project } from "./data";
import { GraphSimulation, type SimulationNode } from "./graph-simulation";

export type GraphControls = {
  zoom: number;
  pan: THREE.Vector2;
  paused: boolean;
  shake: number;
  reset: number;
  autoFit: boolean;
  selected?: number;
  focusedDomain?: number;
};

type Visual = {
  node: SimulationNode;
  project?: Project;
  group: THREE.Group;
  core: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>;
  halos: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>[];
  rim: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>;
  focus: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>;
  selected: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>;
  label: HTMLButtonElement;
  labelWidth: number;
  grow: number;
  life: number;
  labelAlpha: number;
  retiring: boolean;
  x: number;
  y: number;
  radius: number;
  displayX: number;
  displayY: number;
  illumination: number;
};
type EdgeView = {
  a: Visual;
  b: Visual;
  kind: string;
  line: THREE.Line<
    THREE.BufferGeometry,
    THREE.LineBasicMaterial | THREE.LineDashedMaterial
  >;
  particles: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>[];
  opacity: number;
};
const clamp = THREE.MathUtils.clamp;
const smoothstep = THREE.MathUtils.smoothstep;
const ease = (value: number, target: number, dt: number, rate: number) =>
  value + (target - value) * (1 - Math.exp(-dt * rate));

/** Rendering and gestures mirror src/screens/map.rs. Physics lives in graph-simulation.ts. */
export function createGraphScene(
  host: HTMLElement,
  labelLayer: HTMLElement,
  icons: HTMLElement[],
  initialProjects: Project[],
  initialNames: string[],
  control: GraphControls,
  onSelect: (p: Project) => void,
  onDomain: (i: number) => void,
  onTourFocus: (domain?: number) => void = () => {},
) {
  const renderer = new THREE.WebGLRenderer({
    antialias: true,
    alpha: true,
    powerPreference: "high-performance",
  });
  renderer.setClearColor(0, 0);
  host.appendChild(renderer.domElement);
  const canvas = renderer.domElement;
  const interactionRoot = host.parentElement!;
  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 100);
  camera.position.z = 10;
  const disk = new THREE.CircleGeometry(1, 40);
  const ring = new THREE.RingGeometry(0.94, 1, 56);
  const sim = new GraphSimulation();
  const visuals = new Map<string, Visual>();
  let ordered: Visual[] = [],
    edges: EdgeView[] = [],
    names = initialNames;
  let width = 1,
    height = 1,
    frame = 0,
    previous = 0,
    elapsed = 0;
  let visible = true,
    destroyed = false,
    first = true;
  let hovered: string | undefined,
    lastHover: string | undefined,
    highlight = 0;
  let seenDomain = control.focusedDomain,
    lastDomain = control.focusedDomain,
    domainFade = 0;
  let lastShake = control.shake,
    lastReset = control.reset;
  let pointerInside = false,
    lastInteraction = -Infinity;
  let tour: string[] = [];
  let tourTime = 0,
    presentedTourDomain: number | undefined;
  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  let down:
    | {
        clientX: number;
        clientY: number;
        worldX: number;
        worldY: number;
        panX: number;
        panY: number;
        zoom: number;
        node?: Visual;
        offsetX: number;
        offsetY: number;
        moved: boolean;
      }
    | undefined;
  let zoomAnchor:
    { x: number; y: number; offsetX: number; offsetY: number } | undefined;

  function mesh(color: string, geometry: THREE.BufferGeometry = disk) {
    return new THREE.Mesh(
      geometry,
      new THREE.MeshBasicMaterial({
        color,
        transparent: true,
        opacity: 0,
        depthWrite: false,
      }),
    );
  }
  function disposeVisual(v: Visual) {
    v.group.traverse((o) => {
      if (o instanceof THREE.Mesh) o.material.dispose();
    });
    scene.remove(v.group);
    v.label.remove();
  }
  function makeVisual(node: SimulationNode, project?: Project): Visual {
    const group = new THREE.Group();
    const color = colors[node.domain];
    const halos = [mesh(color), mesh(color)];
    const core = mesh(
      project
        ? color
        : new THREE.Color("#181818")
            .lerp(new THREE.Color(color), 0.22)
            .getStyle(),
    );
    const rim = mesh(project ? "#ffffff" : color, ring);
    const focus = mesh(project ? "#f4a945" : color, ring);
    const selected = mesh("#fbf8f3", ring);
    group.add(...halos, core, rim, focus, selected);
    core.position.z = 0.2;
    rim.position.z = 0.3;
    focus.position.z = 0.4;
    selected.position.z = 0.5;
    scene.add(group);
    const label = document.createElement("button");
    label.className = project ? "graph-project-label" : "graph-domain-label";
    label.style.left = "0";
    label.style.top = "0";
    label.style.setProperty("--domain-color", color);
    labelLayer.appendChild(label);
    const v: Visual = {
      node,
      project,
      group,
      core,
      halos,
      rim,
      focus,
      selected,
      label,
      labelWidth: 0,
      grow: 0,
      life: 0,
      labelAlpha: 0,
      retiring: false,
      x: 0,
      y: 0,
      radius: 0,
      displayX: node.x,
      displayY: node.y,
      illumination: 1,
    };
    label.onpointerenter = () => {
      hovered = node.key;
    };
    label.onpointerleave = () => {
      if (!down) hovered = undefined;
    };
    label.onfocus = () => {
      hovered = node.key;
    };
    label.onblur = () => {
      if (!down) hovered = undefined;
    };
    label.onpointerdown = (e) => start(e, v);
    label.onclick = (e) => {
      if (e.detail === 0) activate(v);
    };
    return v;
  }
  function clearEdges() {
    edges.forEach((e) => {
      e.line.geometry.dispose();
      e.line.material.dispose();
      scene.remove(e.line);
      e.particles.forEach((p) => {
        p.material.dispose();
        scene.remove(p);
      });
    });
    edges = [];
  }
  function update(projects: Project[], nextNames: string[]) {
    names = nextNames;
    const changed = sim.sync(projects);
    const byId = new Map(projects.map((p) => [p.id, p]));
    visuals.forEach((v) => {
      v.retiring = true;
    });
    ordered = sim.nodes.map((node) => {
      const project =
        node.projectId === undefined ? undefined : byId.get(node.projectId);
      let v = visuals.get(node.key);
      if (!v) {
        v = makeVisual(node, project);
        visuals.set(node.key, v);
      }
      v.node = node;
      v.project = project;
      v.retiring = false;
      const color = colors[node.domain];
      v.core.material.color.set(
        project
          ? color
          : new THREE.Color("#181818")
              .lerp(new THREE.Color(color), 0.22)
              .getStyle(),
      );
      v.halos.forEach((halo) => halo.material.color.set(color));
      v.rim.material.color.set(project ? "#ffffff" : color);
      v.focus.material.color.set(project ? "#f4a945" : color);
      v.label.style.setProperty("--domain-color", color);
      v.label.textContent = project
        ? project.name
        : `${names[node.domain]} ${projects.filter((p) => p.domain === node.domain).length}`;
      v.labelWidth = v.label.offsetWidth;
      return v;
    });
    if (changed) {
      if (down?.node) sim.unpin(down.node.node.key);
      down = undefined;
      if (hovered && !ordered.some((v) => v.node.key === hovered))
        hovered = undefined;
      clearEdges();
      edges = sim.edges.map((edge) => {
        const a = ordered[edge.a],
          b = ordered[edge.b];
        const primary = edge.kind === "domain" || edge.kind === "alsoDomain";
        const color = primary
          ? colors[a.node.domain]
          : edge.kind === "folder"
            ? "#ece7df"
            : edge.kind === "tags"
              ? "#8bbbd8"
              : "#f4a945";
        const options = {
          color,
          transparent: true,
          opacity: 0,
          depthWrite: false,
        };
        const mat = primary
          ? new THREE.LineBasicMaterial(options)
          : new THREE.LineDashedMaterial({
              ...options,
              dashSize: 5,
              gapSize: 4,
            });
        const geometry = new THREE.BufferGeometry();
        geometry.setAttribute(
          "position",
          new THREE.BufferAttribute(new Float32Array(6), 3).setUsage(
            THREE.DynamicDrawUsage,
          ),
        );
        geometry.setAttribute(
          "lineDistance",
          new THREE.BufferAttribute(new Float32Array(2), 1).setUsage(
            THREE.DynamicDrawUsage,
          ),
        );
        const line = new THREE.Line(geometry, mat);
        line.frustumCulled = false;
        scene.add(line);
        const particles = [mesh(color), mesh(color)];
        particles.forEach((p) => {
          p.position.z = 0.6;
          scene.add(p);
        });
        return { a, b, kind: edge.kind, line, particles, opacity: 0 };
      });
      control.autoFit = true;
      // Showcase a strongly connected project from each domain, not random isolated nodes.
      tour = [...new Set(ordered.map((v) => v.node.domain))].flatMap(
        (domain) => {
          const candidates = ordered.filter(
            (v) => v.project && v.node.domain === domain,
          );
          candidates.sort(
            (a, b) =>
              sim.neighbors[ordered.indexOf(b)].length -
              sim.neighbors[ordered.indexOf(a)].length,
          );
          return candidates[0] ? [candidates[0].node.key] : [];
        },
      );
      if (reduced.matches) sim.settle();
    }
    return edges.length;
  }
  function resize() {
    width = Math.max(host.clientWidth, 1);
    height = Math.max(host.clientHeight, 1);
    const scale = host.getBoundingClientRect().width / width;
    const dotRadius = Math.max(0.7, 0.4 / Math.max(scale, 0.1));
    interactionRoot.style.backgroundImage = `radial-gradient(#ffffff10 ${dotRadius}px, transparent ${dotRadius + 0.25}px)`;
    renderer.setPixelRatio(Math.min(devicePixelRatio * scale, 2));
    renderer.setSize(width, height, false);
    camera.left = -width / 2;
    camera.right = width / 2;
    camera.top = height / 2;
    camera.bottom = -height / 2;
    camera.updateProjectionMatrix();
  }
  function point(e: PointerEvent | WheelEvent) {
    const rect = host.getBoundingClientRect();
    const x = ((e.clientX - rect.left) * width) / rect.width - width / 2;
    const y = ((e.clientY - rect.top) * height) / rect.height - height / 2;
    return {
      x: camera.position.x + x / camera.zoom,
      y: -camera.position.y + y / camera.zoom,
      offsetX: x,
      offsetY: y,
    };
  }
  function pick(p: ReturnType<typeof point>) {
    let found: Visual | undefined,
      distance = Infinity;
    ordered.forEach((v) => {
      const d = Math.hypot(v.group.position.x - p.x, -v.group.position.y - p.y);
      if (
        d * camera.zoom < Math.max(v.radius * camera.zoom + 5, 10) &&
        d < distance
      ) {
        found = v;
        distance = d;
      }
    });
    return found;
  }
  function activate(v: Visual) {
    if (v.project) onSelect(v.project);
    else onDomain(v.node.domain);
  }
  function start(e: PointerEvent, forced?: Visual) {
    if (e.button !== 0) return;
    e.preventDefault();
    const p = point(e),
      node = forced ?? pick(p);
    zoomAnchor = undefined;
    down = {
      clientX: e.clientX,
      clientY: e.clientY,
      worldX: p.x,
      worldY: p.y,
      panX: camera.position.x,
      panY: -camera.position.y,
      zoom: camera.zoom,
      node,
      offsetX: node ? node.node.x - p.x : 0,
      offsetY: node ? node.node.y - p.y : 0,
      moved: false,
    };
    canvas.setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    const p = point(e);
    if (!down) {
      hovered = pick(p)?.node.key;
      host.style.cursor = hovered ? "pointer" : "grab";
      return;
    }
    if (
      !down.moved &&
      Math.hypot(e.clientX - down.clientX, e.clientY - down.clientY) > 3
    ) {
      down.moved = true;
      control.pan.set(camera.position.x, -camera.position.y);
      control.zoom = camera.zoom;
    }
    if (!down.moved) return;
    host.style.cursor = "grabbing";
    control.autoFit = false;
    if (down.node) {
      hovered = down.node.node.key;
      sim.pin(down.node.node.key, p.x + down.offsetX, p.y + down.offsetY);
    } else {
      const rect = host.getBoundingClientRect();
      control.pan.set(
        down.panX -
          ((e.clientX - down.clientX) * width) / rect.width / down.zoom,
        down.panY -
          ((e.clientY - down.clientY) * height) / rect.height / down.zoom,
      );
      camera.position.x = control.pan.x;
      camera.position.y = -control.pan.y;
    }
  }
  function end(e: PointerEvent) {
    const gesture = down;
    down = undefined;
    if (gesture?.node) {
      sim.unpin(gesture.node.node.key);
      if (!gesture.moved && e.type !== "pointercancel") activate(gesture.node);
    }
    if (canvas.hasPointerCapture(e.pointerId))
      canvas.releasePointerCapture(e.pointerId);
    hovered = pick(point(e))?.node.key;
    host.style.cursor = hovered ? "pointer" : "grab";
  }
  function wheel(e: WheelEvent) {
    e.preventDefault();
    const p = point(e);
    const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? height : 1;
    control.zoom = clamp(
      control.zoom * Math.exp(-e.deltaY * unit * 0.0018),
      0.2,
      4,
    );
    control.autoFit = false;
    zoomAnchor = p;
    control.pan.set(
      p.x - p.offsetX / control.zoom,
      p.y - p.offsetY / control.zoom,
    );
  }
  function reset() {
    control.autoFit = true;
    control.reset++;
  }
  function leave() {
    if (!down) hovered = undefined;
  }
  function entered() {
    pointerInside = true;
    lastInteraction = elapsed;
    if (presentedTourDomain !== undefined) {
      control.autoFit = false;
      control.pan.set(camera.position.x, -camera.position.y);
      control.zoom = camera.zoom;
    }
  }
  function exited() {
    pointerInside = false;
    lastInteraction = elapsed;
    if (!down) hovered = undefined;
  }
  function interacted() {
    lastInteraction = elapsed;
  }
  const observer = new ResizeObserver(resize);
  observer.observe(host);
  // An ancestor transform changes display density without resizing the host.
  const scaledScreen = host.closest(".mac-screen");
  if (scaledScreen) observer.observe(scaledScreen);
  window.addEventListener("resize", resize);
  const intersection = new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    wake();
  });
  intersection.observe(host);
  function wake() {
    if (!frame && !destroyed && visible && !document.hidden) {
      previous = 0;
      frame = requestAnimationFrame(render);
    }
  }
  document.addEventListener("visibilitychange", wake);
  function motionChanged() {
    if (reduced.matches) sim.settle();
    wake();
  }
  reduced.addEventListener("change", motionChanged);
  canvas.addEventListener("pointerdown", start);
  canvas.addEventListener("pointermove", move);
  canvas.addEventListener("pointerup", end);
  canvas.addEventListener("pointercancel", end);
  canvas.addEventListener("pointerleave", leave);
  interactionRoot.addEventListener("wheel", wheel, { passive: false });
  interactionRoot.addEventListener("pointerenter", entered);
  interactionRoot.addEventListener("pointerleave", exited);
  interactionRoot.addEventListener("pointerdown", interacted);
  interactionRoot.addEventListener("focusin", interacted);
  canvas.addEventListener("dblclick", reset);

  function render(now: number) {
    frame = 0;
    if (destroyed || !visible || document.hidden) return;
    const dt = previous ? Math.min((now - previous) / 1000, 0.05) : 1 / 60;
    previous = now;
    const animate = !control.paused && !reduced.matches;
    if (animate) {
      sim.step(dt);
      elapsed += dt;
    }
    if (lastShake !== control.shake) {
      sim.shake();
      if (reduced.matches) sim.settle();
      lastShake = control.shake;
    }
    if (seenDomain !== control.focusedDomain || lastReset !== control.reset) {
      control.autoFit = true;
      zoomAnchor = undefined;
      seenDomain = control.focusedDomain;
      lastReset = control.reset;
    }
    const autoplay =
      animate &&
      control.autoFit &&
      !pointerInside &&
      !hovered &&
      elapsed - lastInteraction > 6 &&
      control.selected === undefined &&
      control.focusedDomain === undefined;
    if (autoplay) tourTime += dt;
    const tourPhase = tourTime % 10;
    const tourProject =
      tour[Math.floor(tourTime / 10) % Math.max(1, tour.length)];
    const tourDomain =
      autoplay && tourPhase >= 1 && tourPhase < 7.5
        ? visuals.get(tourProject)?.node.domain
        : undefined;
    const effectiveDomain = control.focusedDomain ?? tourDomain;
    if (tourDomain !== presentedTourDomain) {
      presentedTourDomain = tourDomain;
      onTourFocus(tourDomain);
    }
    if (effectiveDomain !== undefined) lastDomain = effectiveDomain;
    if (control.autoFit && !down) {
      const target = sim.fit(width, height, effectiveDomain);
      if (target) {
        control.pan.set(target.x, target.y);
        control.zoom = target.zoom;
      }
    }
    const cameraRate = reduced.matches || first ? 1 : 1 - Math.exp(-dt * 5);
    camera.zoom = Math.exp(
      THREE.MathUtils.lerp(
        Math.log(camera.zoom),
        Math.log(control.zoom),
        cameraRate,
      ),
    );
    if (zoomAnchor && !control.autoFit) {
      camera.position.x = zoomAnchor.x - zoomAnchor.offsetX / camera.zoom;
      camera.position.y = -(zoomAnchor.y - zoomAnchor.offsetY / camera.zoom);
      if (Math.abs(Math.log(camera.zoom / control.zoom)) < 0.0001)
        zoomAnchor = undefined;
    } else {
      camera.position.x += (control.pan.x - camera.position.x) * cameraRate;
      camera.position.y += (-control.pan.y - camera.position.y) * cameraRate;
    }
    first = false;
    camera.updateProjectionMatrix();
    camera.updateMatrixWorld();
    const autoSpot =
      tourDomain !== undefined
        ? tourPhase < 4 || tourPhase > 6.6
          ? `domain:${tourDomain}`
          : tourProject
        : undefined;
    const spot =
      down?.moved && down.node ? down.node.node.key : (hovered ?? autoSpot);
    if (spot) lastHover = spot;
    highlight = ease(highlight, spot ? 1 : 0, dt, 18);
    domainFade = ease(
      domainFade,
      effectiveDomain === undefined ? 0 : 1,
      dt,
      12,
    );
    const hoverIndex = ordered.findIndex((v) => v.node.key === lastHover);
    const lit = new Set<number>(
      hoverIndex < 0 ? [] : [hoverIndex, ...sim.neighbors[hoverIndex]],
    );
    for (const v of ordered) {
      const index = ordered.indexOf(v);
      const domainAlpha =
        lastDomain !== undefined &&
        v.node.domain !== lastDomain &&
        !v.node.also.includes(lastDomain)
          ? 1 - 0.82 * domainFade
          : 1;
      const target =
        domainAlpha *
        (hoverIndex >= 0 && !lit.has(index) ? 1 - 0.72 * highlight : 1);
      v.illumination = ease(v.illumination, target, dt, 18);
    }
    const fade = (v: Visual) => v.illumination * v.life;
    visuals.forEach((v, key) => {
      const n = v.node,
        hub = !v.project,
        selected = v.project?.id === control.selected && !!v.project;
      v.life = reduced.matches
        ? v.retiring
          ? 0
          : 1
        : ease(v.life, v.retiring ? 0 : 1, dt, 14);
      if (v.retiring && v.life < 0.01) {
        disposeVisual(v);
        visuals.delete(key);
        return;
      }
      v.grow = ease(v.grow, spot === key ? 1 : 0, dt, 22);
      const drift = down?.node === v ? 0 : hub ? 0.8 : 1.6;
      const index = Math.max(0, ordered.indexOf(v));
      v.displayX =
        n.fixed || reduced.matches ? n.x : ease(v.displayX, n.x, dt, 30);
      v.displayY =
        n.fixed || reduced.matches ? n.y : ease(v.displayY, n.y, dt, 30);
      const x =
        v.displayX +
        (reduced.matches ? 0 : Math.sin(elapsed * 0.8 + index * 1.7) * drift);
      const y =
        v.displayY +
        (reduced.matches ? 0 : Math.cos(elapsed * 0.63 + index * 2.3) * drift);
      v.group.position.set(x, -y, 0);
      v.x = (x - camera.position.x) * camera.zoom + width / 2;
      v.y = (y + camera.position.y) * camera.zoom + height / 2;
      const r =
        Math.max(n.radius, (hub ? 12 : 3.5) / camera.zoom) *
        (1 + (hub ? 0.12 : 0.35) * v.grow);
      v.radius = r;
      const alpha = fade(v),
        glow = Math.max(v.grow, selected ? 0.8 : 0);
      if (v.retiring) {
        v.label.style.opacity = `${v.life * v.labelAlpha}`;
        v.label.style.pointerEvents = "none";
        v.label.tabIndex = -1;
      }
      const status =
        v.project?.status === "Archived"
          ? 0.3
          : v.project?.attention === "Reserve"
            ? 0.45
            : 1;
      v.core.scale.setScalar(r);
      v.core.material.opacity = alpha * (hub ? 1 : status);
      v.rim.scale.setScalar(r);
      v.rim.material.opacity = alpha * (hub ? 0.9 : 0.16 * status);
      v.halos.forEach((halo, i) => {
        halo.scale.setScalar(r * (hub ? [2.2, 1.55][i] : [2.5, 1.7][i]));
        halo.material.opacity =
          alpha *
          (hub
            ? [0.045 + 0.04 * v.grow, 0.08 + 0.06 * v.grow][i]
            : [0.07, 0.14][i] * glow);
      });
      v.focus.scale.setScalar(r + (hub ? 5 : 3.5) / camera.zoom);
      v.focus.material.opacity =
        alpha *
        (hub
          ? effectiveDomain === n.domain
            ? 1
            : 0
          : v.project?.attention === "Focus" && v.project.status !== "Archived"
            ? 1
            : 0);
      v.selected.scale.setScalar(r + 7 / camera.zoom);
      v.selected.material.opacity = selected ? Math.max(alpha, 0.5) : 0;
      for (const shape of [...v.halos, v.core, v.rim, v.focus, v.selected])
        shape.visible = shape.material.opacity > 0.005;
      if (hub) {
        const icon = icons[n.domain];
        if (icon) {
          icon.style.display = "flex";
          icon.style.color = colors[n.domain];
          icon.style.transform = `translate3d(${v.x}px,${v.y}px,0) translate(-50%,-50%) scale(${clamp((r * camera.zoom) / 25, 0.5, 1.12)})`;
          icon.style.left = "0";
          icon.style.top = "0";
          icon.style.opacity = `${alpha}`;
        }
      }
    });
    icons.forEach((icon, i) => {
      if (!ordered.some((v) => !v.project && v.node.domain === i))
        icon.style.display = "none";
    });
    edges.forEach((e, i) => {
      const hoverLit = e.a.node.key === lastHover || e.b.node.key === lastHover;
      const openLit =
        (!!e.a.project && e.a.project.id === control.selected) ||
        (!!e.b.project && e.b.project.id === control.selected);
      const amount = Math.max(hoverLit ? highlight : 0, openLit ? 0.75 : 0);
      // Keep the cross-domain structure legible in the larger public demo.
      const base =
        e.kind === "domain" ? 0.1 : e.kind === "folder" ? 0.075 : 0.045;
      const bright =
        e.kind === "domain" ? 0.85 : e.kind === "folder" ? 0.8 : 0.7;
      e.opacity = ease(
        e.opacity,
        (base + (bright - base) * amount) * Math.min(fade(e.a), fade(e.b)),
        dt,
        22,
      );
      e.line.material.opacity = e.opacity;
      e.line.visible = e.opacity > 0.005;
      e.line.renderOrder = amount > 0.1 ? 1 : 0;
      const position = e.line.geometry.getAttribute(
        "position",
      ) as THREE.BufferAttribute;
      position.setXYZ(0, e.a.group.position.x, e.a.group.position.y, 0);
      position.setXYZ(1, e.b.group.position.x, e.b.group.position.y, 0);
      position.needsUpdate = true;
      const distance = e.line.geometry.getAttribute(
        "lineDistance",
      ) as THREE.BufferAttribute;
      distance.setX(1, e.a.group.position.distanceTo(e.b.group.position));
      distance.needsUpdate = true;
      if (e.line.material instanceof THREE.LineDashedMaterial) {
        e.line.material.dashSize = 5 / camera.zoom;
        e.line.material.gapSize = 4 / camera.zoom;
      }
      e.particles.forEach((particle, j) => {
        const f = (elapsed * 0.5 + i * 0.37 + j * 0.5) % 1;
        const reversed =
          !["domain", "alsoDomain"].includes(e.kind) &&
          lastHover === e.b.node.key;
        particle.position.lerpVectors(
          reversed ? e.b.group.position : e.a.group.position,
          reversed ? e.a.group.position : e.b.group.position,
          f,
        );
        particle.position.z = 0.6;
        particle.scale.setScalar(2.2 / camera.zoom);
        particle.material.opacity =
          hoverLit && !reduced.matches
            ? 0.9 *
              highlight *
              (1 - Math.abs(f - 0.5)) *
              Math.min(fade(e.a), fade(e.b))
            : 0;
        particle.visible = particle.material.opacity > 0.01;
      });
    });
    // DOM labels move by transforms; collision decisions fade instead of snapping visibility.
    const boxes: { x: number; y: number; w: number; h: number }[] = [];
    const priority = (v: Visual) =>
      v.node.key === spot
        ? 4
        : v.project?.id === control.selected
          ? 3
          : lit.has(ordered.indexOf(v)) && highlight > 0.3
            ? 2
            : v.project?.attention === "Focus"
              ? 1
              : 0;
    const labelOrder = [...ordered].sort(
      (a, b) =>
        Number(!!a.project) - Number(!!b.project) ||
        priority(b) - priority(a) ||
        b.node.radius - a.node.radius,
    );
    labelOrder.forEach((v) => {
      const hub = !v.project,
        p = priority(v),
        r = v.radius * camera.zoom;
      const ringOffset = p === 3 ? 9 : v.project?.attention === "Focus" ? 6 : 0;
      const x = v.x + (hub ? -v.labelWidth / 2 : r + ringOffset + 7);
      const y = v.y + (hub ? r + 8 : -10);
      const h = hub ? 24 : 20;
      const box = { x, y, w: v.labelWidth, h };
      const collides = boxes.some(
        (b) =>
          x < b.x + b.w + 4 &&
          x + box.w + 4 > b.x &&
          y < b.y + b.h + 2 &&
          y + h + 2 > b.y,
      );
      const inBounds =
        x >= 4 && x + box.w <= width - 4 && y >= 42 && y + h <= height - 36;
      const near = smoothstep(camera.zoom, 0.55, 0.7);
      const target = hub
        ? Math.max(0.25, fade(v))
        : p >= 3 || (!collides && inBounds)
          ? (p >= 1
              ? 1
              : near * (v.project?.attention === "Reserve" ? 0.65 : 1)) *
            fade(v)
          : 0;
      v.labelAlpha = ease(v.labelAlpha, target, dt, 20);
      v.label.style.transform = `translate3d(${x}px,${y}px,0)`;
      v.label.style.opacity = `${v.labelAlpha}`;
      v.label.style.pointerEvents = target > 0.1 ? "auto" : "none";
      v.label.tabIndex = target > 0.1 ? 0 : -1;
      if (target > 0.05) boxes.push(box);
      if (hub) boxes.push({ x: v.x - r, y: v.y - r, w: r * 2, h: r * 2 });
    });
    let spacing = 28 * camera.zoom;
    while (spacing < 14) spacing *= 2;
    const container = host.parentElement!;
    container.style.backgroundSize = `${spacing}px ${spacing}px`;
    container.style.backgroundPosition = `${width / 2 - camera.position.x * camera.zoom}px ${height / 2 + camera.position.y * camera.zoom}px`;
    renderer.render(scene, camera);
    frame = requestAnimationFrame(render);
  }
  resize();
  update(initialProjects, initialNames);
  wake();
  return {
    update,
    destroy() {
      onTourFocus(undefined);
      destroyed = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      intersection.disconnect();
      document.removeEventListener("visibilitychange", wake);
      window.removeEventListener("resize", resize);
      reduced.removeEventListener("change", motionChanged);
      canvas.removeEventListener("pointerdown", start);
      canvas.removeEventListener("pointermove", move);
      canvas.removeEventListener("pointerup", end);
      canvas.removeEventListener("pointercancel", end);
      canvas.removeEventListener("pointerleave", leave);
      interactionRoot.removeEventListener("wheel", wheel);
      interactionRoot.removeEventListener("pointerenter", entered);
      interactionRoot.removeEventListener("pointerleave", exited);
      interactionRoot.removeEventListener("pointerdown", interacted);
      interactionRoot.removeEventListener("focusin", interacted);
      canvas.removeEventListener("dblclick", reset);
      clearEdges();
      visuals.forEach(disposeVisual);
      disk.dispose();
      ring.dispose();
      renderer.dispose();
      canvas.remove();
      icons.forEach((icon) => {
        icon.style.display = "none";
      });
    },
  };
}
