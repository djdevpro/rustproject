import fixtures from "./fixtures.json";
import type { Project } from "./data";

/** Port of src/graph.rs. The world uses the native application's +y-down axes. */
export const ALPHA_MIN = 0.001;
const ALPHA_DECAY = 0.0228;
const VELOCITY_RETAINED = 0.6;
const STEP = 1 / 60;

export type EdgeKind = "domain" | "alsoDomain" | "folder" | "stack" | "tags";
export type SimulationNode = {
  key: string;
  index: number;
  projectId?: number;
  domain: number;
  also: number[];
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  mass: number;
  charge: number;
  fixed: boolean;
};
export type SimulationEdge = {
  a: number;
  b: number;
  kind: EdgeKind;
  rest: number;
  strength: number;
  bias: number;
};
export type SimulationContext = {
  domains: { name: string }[];
  config: { roots: { path: string }[] };
  ontology: {
    facets: { id: number; name: string }[];
    concepts: {
      id: number;
      facet: number;
      name: string;
      parent: number | null;
    }[];
  };
};
const projectRadius = (days: number) => (days <= 7 ? 8 : days <= 30 ? 6.5 : 5);
const pathKey = (path: string) =>
  path.replaceAll("\\", "/").replace(/\/+$/, "").toLowerCase();
const parentPath = (path: string) => {
  const p = pathKey(path);
  return p.slice(0, Math.max(0, p.lastIndexOf("/")));
};
const nameKey = (name: string) => name.trim().toLowerCase();
function jiggle(i: number, j: number) {
  const h = ((Math.imul(i, 7919) ^ Math.imul(j, 104729)) >>> 0) % 1000;
  return { x: Math.cos(h * 0.006283) * 0.5, y: Math.sin(h * 0.006283) * 0.5 };
}

export class GraphSimulation {
  nodes: SimulationNode[] = [];
  edges: SimulationEdge[] = [];
  neighbors: number[][] = [];
  alpha = 0;
  alphaTarget = 0;
  private signature = "";
  private accumulator = 0;
  private context: SimulationContext;

  constructor(context: SimulationContext = fixtures) {
    this.context = context;
  }
  get warm() {
    return this.alpha > ALPHA_MIN || this.alphaTarget > 0;
  }

  sync(projects: Project[], context = this.context): boolean {
    this.context = context;
    // Stable ordering makes the force solve independent of library sort order.
    const ordered = projects
      .map((project, index) => ({ project, index }))
      .sort((a, b) => a.project.id - b.project.id);
    const signature = JSON.stringify([
      context,
      ordered.map(({ project: p }) => [
        p.id,
        p.domain,
        p.stack,
        p.folders[0],
        projectRadius(p.idle_days),
        p.tags.filter((t) => t.weight >= 0.5).map((t) => t.concept),
      ]),
    ]);
    if (signature === this.signature) {
      const indexes = new Map(projects.map((p, i) => [p.id, i]));
      for (const node of this.nodes)
        if (node.projectId !== undefined)
          node.index = indexes.get(node.projectId)!;
      return false;
    }
    const first = !this.signature || !this.nodes.length;
    this.signature = signature;
    const old = new Map(this.nodes.map((n) => [n.key, n]));
    const perDomain = new Map<number, typeof ordered>();
    const facet = context.ontology.facets.find((f) =>
      ["domain", "domaine"].includes(nameKey(f.name)),
    )?.id;
    const concepts = new Map(context.ontology.concepts.map((c) => [c.id, c]));
    const domainOf = (id: number): number | undefined => {
      let concept = concepts.get(id);
      if (!concept || concept.facet !== facet) return undefined;
      for (let i = 0; i < 8 && concept.parent !== null; i++) {
        const parent = concepts.get(concept.parent);
        if (!parent) break;
        concept = parent;
      }
      const index = context.domains.findIndex(
        (d) => nameKey(d.name) === nameKey(concept!.name),
      );
      return index < 0 ? undefined : index;
    };
    const alsoOf = new Map<number, number[]>();
    for (const item of ordered) {
      const d = Math.max(
        0,
        Math.min(item.project.domain, context.domains.length - 1),
      );
      if (!perDomain.has(d)) perDomain.set(d, []);
      perDomain.get(d)!.push(item);
      const also = [
        ...new Set(
          item.project.tags
            .filter((t) => t.weight >= 0.5)
            .map((t) => domainOf(t.concept))
            .filter((a): a is number => a !== undefined && a !== d),
        ),
      ];
      alsoOf.set(item.project.id, also);
      for (const other of also)
        if (!perDomain.has(other)) perDomain.set(other, []);
    }
    const groups = [...perDomain].sort(([a], [b]) => a - b);
    const count = Math.max(1, groups.length);
    const ring = 150 + 70 * Math.sqrt(count);
    const nodes: SimulationNode[] = [];
    const hubs = new Map<number, number>();
    const projectNodes = new Map<number, number>();
    const add = (
      key: string,
      index: number,
      domain: number,
      x: number,
      y: number,
      radius: number,
      charge: number,
      projectId?: number,
    ) => {
      const existing = old.get(key);
      nodes.push({
        key,
        index,
        domain,
        projectId,
        also: projectId === undefined ? [] : alsoOf.get(projectId)!,
        x: existing?.x ?? x,
        y: existing?.y ?? y,
        vx: existing?.vx ?? 0,
        vy: existing?.vy ?? 0,
        radius,
        mass: radius * radius,
        charge,
        fixed: existing?.fixed ?? false,
      });
    };
    groups.forEach(([d, members], k) => {
      const a = (k * Math.PI * 2) / count - 1.2;
      hubs.set(d, nodes.length);
      add(
        `domain:${d}`,
        d,
        d,
        Math.cos(a) * (count > 1 ? ring : 0),
        Math.sin(a) * (count > 1 ? ring : 0),
        16 + 2.2 * Math.sqrt(members.length),
        -(220 + 18 * members.length),
      );
    });
    for (const [d, members] of groups) {
      const hub = nodes[hubs.get(d)!];
      members.forEach(({ project: p, index }, j) => {
        const r =
          hub.radius + (first ? 4 + 2 * Math.sqrt(j) : 18 + 9 * Math.sqrt(j));
        const angle = j * 2.399963 + d;
        projectNodes.set(p.id, nodes.length);
        add(
          `project:${p.id}`,
          index,
          d,
          hub.x + Math.cos(angle) * r,
          hub.y + Math.sin(angle) * r,
          projectRadius(p.idle_days),
          -45,
          p.id,
        );
      });
    }
    const edges: Omit<SimulationEdge, "bias">[] = [];
    const addEdge = (
      a: number,
      b: number,
      kind: EdgeKind,
      rest: number,
      strength: number,
    ) => {
      if (a !== b) edges.push({ a, b, kind, rest, strength });
    };
    for (const [d, members] of groups) {
      const hub = hubs.get(d)!;
      const rest = nodes[hub].radius + 34 + 3 * Math.sqrt(members.length);
      for (const { project } of members)
        addEdge(hub, projectNodes.get(project.id)!, "domain", rest, 0.7);
    }
    for (const [id, list] of alsoOf)
      for (const d of list)
        addEdge(hubs.get(d)!, projectNodes.get(id)!, "alsoDomain", 150, 0.02);
    const roots = new Set(context.config.roots.map((r) => pathKey(r.path)));
    const folderNodes = new Map(
      ordered
        .filter(({ project: p }) => p.folders[0])
        .map(({ project: p }) => [
          pathKey(p.folders[0]),
          projectNodes.get(p.id)!,
        ]),
    );
    const byParent = new Map<string, number[]>();
    for (const { project: p } of ordered) {
      if (!p.folders[0]) continue;
      const parent = parentPath(p.folders[0]);
      if (!parent) continue;
      const node = projectNodes.get(p.id)!;
      const owner = folderNodes.get(parent);
      if (owner !== undefined) addEdge(owner, node, "folder", 40, 0.5);
      else if (!roots.has(parent)) {
        if (!byParent.has(parent)) byParent.set(parent, []);
        byParent.get(parent)!.push(node);
      }
    }
    for (const group of byParent.values())
      if (group.length >= 2 && group.length <= 12)
        group.forEach((a, i) => {
          if (group.length > 2 || i === 0)
            addEdge(a, group[(i + 1) % group.length], "folder", 46, 0.25);
        });
    for (const edge of edges)
      if (
        edge.kind === "folder" &&
        nodes[edge.a].domain !== nodes[edge.b].domain
      ) {
        edge.rest = 110;
        edge.strength = 0.02;
      }
    const stacks = nodes.map((n) =>
      n.projectId === undefined
        ? []
        : projects[n.index].stack.map((s) => s.toLowerCase()),
    );
    const tags = nodes.map((n) =>
      n.projectId === undefined
        ? []
        : projects[n.index].tags
            .filter((t) => t.weight >= 0.5)
            .filter((t) => {
              const c = concepts.get(t.concept);
              return c && !(c.facet === facet && c.parent === null);
            })
            .map((t) => t.concept),
    );
    const df = new Map<number, number>();
    for (const set of tags)
      for (const c of set) df.set(c, (df.get(c) ?? 0) + 1);
    const projectCount = Math.max(1, tags.filter((t) => t.length).length);
    const idf = (c: number) => Math.log(1 + projectCount / (df.get(c) ?? 1));
    const similarities = (kind: "stack" | "tags") => {
      const best: { score: number; other: number }[][] = nodes.map(() => []);
      for (let a = 0; a < nodes.length; a++)
        for (let b = a + 1; b < nodes.length; b++) {
          if (
            nodes[a].projectId === undefined ||
            nodes[b].projectId === undefined
          )
            continue;
          if (kind === "stack" && nodes[a].domain === nodes[b].domain) continue;
          let score = 0;
          if (kind === "stack") {
            const shared = stacks[a].filter((s) =>
              stacks[b].includes(s),
            ).length;
            if (shared < 2) continue;
            score =
              shared /
              Math.max(1, stacks[a].length + stacks[b].length - shared);
            if (score < 0.5) continue;
          } else {
            const shared = tags[a].filter((t) => tags[b].includes(t));
            if (shared.length < 2) continue;
            const union = [
              ...tags[a],
              ...tags[b].filter((t) => !tags[a].includes(t)),
            ];
            score =
              shared.reduce((s, c) => s + idf(c), 0) /
              Math.max(
                0.001,
                union.reduce((s, c) => s + idf(c), 0),
              );
            if (score < 0.45) continue;
          }
          best[a].push({ score, other: b });
          best[b].push({ score, other: a });
        }
      best.forEach((list, a) =>
        list
          .sort((x, y) => y.score - x.score || x.other - y.other)
          .slice(0, 2)
          .forEach(({ other: b }) => {
            if (a < b)
              addEdge(
                a,
                b,
                kind,
                kind === "stack" ? 170 : 180,
                kind === "stack" ? 0.012 : 0.008,
              );
          }),
      );
    };
    similarities("stack");
    similarities("tags");
    const kinds: EdgeKind[] = [
      "domain",
      "alsoDomain",
      "folder",
      "stack",
      "tags",
    ];
    edges.sort(
      (a, b) =>
        a.a - b.a || a.b - b.b || kinds.indexOf(a.kind) - kinds.indexOf(b.kind),
    );
    const seen = new Set<string>();
    const unique = edges.filter((e) => {
      const key = `${Math.min(e.a, e.b)}:${Math.max(e.a, e.b)}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
    const degree = nodes.map(() => 0);
    this.neighbors = nodes.map(() => []);
    for (const e of unique) {
      degree[e.a]++;
      degree[e.b]++;
      this.neighbors[e.a].push(e.b);
      this.neighbors[e.b].push(e.a);
    }
    this.edges = unique.map((e) => ({
      ...e,
      bias: degree[e.a] / Math.max(1, degree[e.a] + degree[e.b]),
    }));
    this.nodes = nodes;
    this.alpha = first ? 1 : Math.max(0.6, this.alpha);
    this.alphaTarget = nodes.some((n) => n.fixed) ? 0.25 : 0;
    return true;
  }

  tick() {
    this.alpha += (this.alphaTarget - this.alpha) * ALPHA_DECAY;
    const alpha = this.alpha;
    for (const e of this.edges) {
      const a = this.nodes[e.a],
        b = this.nodes[e.b];
      let dx = b.x + b.vx - a.x - a.vx,
        dy = b.y + b.vy - a.y - a.vy;
      if (dx * dx + dy * dy < 1e-6) {
        const j = jiggle(e.a, e.b);
        dx = j.x;
        dy = j.y;
      }
      const distance = Math.hypot(dx, dy);
      const pull = ((distance - e.rest) / distance) * alpha * e.strength;
      b.vx -= dx * pull * e.bias;
      b.vy -= dy * pull * e.bias;
      a.vx += dx * pull * (1 - e.bias);
      a.vy += dy * pull * (1 - e.bias);
    }
    for (let i = 0; i < this.nodes.length; i++)
      for (let j = i + 1; j < this.nodes.length; j++) {
        const a = this.nodes[i],
          b = this.nodes[j];
        let dx = b.x - a.x,
          dy = b.y - a.y,
          l2 = dx * dx + dy * dy;
        if (l2 > 560 * 560) continue;
        if (l2 < 1e-6) {
          const v = jiggle(i, j);
          dx = v.x;
          dy = v.y;
          l2 = dx * dx + dy * dy;
        }
        const soft = Math.max(36, l2);
        a.vx += (dx * b.charge * alpha) / soft;
        a.vy += (dy * b.charge * alpha) / soft;
        b.vx -= (dx * a.charge * alpha) / soft;
        b.vy -= (dy * a.charge * alpha) / soft;
        const radius = a.radius + b.radius + 3;
        if (l2 < radius * radius) {
          const l = Math.sqrt(l2),
            push = ((radius - l) / l) * 0.7;
          const wa = b.mass / (a.mass + b.mass),
            wb = a.mass / (a.mass + b.mass);
          a.vx -= dx * push * wa;
          a.vy -= dy * push * wa;
          b.vx += dx * push * wb;
          b.vy += dy * push * wb;
        }
      }
    for (const node of this.nodes) {
      const gravity = node.projectId === undefined ? 0.05 : 0.02;
      node.vx -= node.x * 0.5 * gravity * alpha;
      node.vy -= node.y * gravity * alpha;
      if (node.fixed) {
        node.vx = 0;
        node.vy = 0;
        continue;
      }
      node.vx *= VELOCITY_RETAINED;
      node.vy *= VELOCITY_RETAINED;
      node.x += node.vx;
      node.y += node.vy;
    }
  }
  /** A bounded fixed timestep keeps forces identical at 30, 60 and 144 Hz. */
  step(dtSeconds: number): number {
    if (!this.warm) {
      this.accumulator = 0;
      return 0;
    }
    this.accumulator += Math.min(0.1, Math.max(0, dtSeconds));
    let ticks = 0;
    while (this.accumulator + 1e-10 >= STEP && ticks < 6 && this.warm) {
      this.tick();
      this.accumulator -= STEP;
      ticks++;
    }
    return ticks;
  }
  reheat(alpha = 0.6) {
    this.alpha = Math.max(this.alpha, alpha);
  }
  shake() {
    this.nodes.forEach((n, i) => {
      const j = jiggle(i, i * 31 + 7),
        force = n.projectId === undefined ? 40 : 70;
      n.vx += j.x * force;
      n.vy += j.y * force;
    });
    this.reheat(0.8);
  }
  pin(key: string, x: number, y: number) {
    const node = this.nodes.find((n) => n.key === key);
    if (!node) return;
    node.fixed = true;
    node.x = x;
    node.y = y;
    node.vx = 0;
    node.vy = 0;
    this.alphaTarget = 0.25;
    this.reheat(0.3);
  }
  unpin(key: string) {
    const node = this.nodes.find((n) => n.key === key);
    if (node) node.fixed = false;
    this.alphaTarget = this.nodes.some((n) => n.fixed) ? 0.25 : 0;
  }
  settle() {
    this.alphaTarget = 0;
    for (let i = 0; i < 800 && this.alpha > ALPHA_MIN; i++) this.tick();
    for (const n of this.nodes) {
      n.vx = 0;
      n.vy = 0;
    }
    this.accumulator = 0;
  }
  fit(
    width: number,
    height: number,
    domain?: number,
  ): { x: number; y: number; zoom: number } | undefined {
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    for (const n of this.nodes) {
      if (domain !== undefined && n.domain !== domain) continue;
      const hub = n.projectId === undefined;
      minX = Math.min(minX, n.x - (hub ? 60 : n.radius));
      minY = Math.min(minY, n.y - n.radius);
      maxX = Math.max(maxX, n.x + (hub ? 60 : n.radius + 90));
      maxY = Math.max(maxY, n.y + n.radius + (hub ? 34 : n.radius));
    }
    if (!Number.isFinite(minX)) return undefined;
    const zoom = Math.max(
      0.25,
      Math.min(
        domain === undefined ? 1.6 : 2.2,
        Math.max(100, width - 60) / Math.max(240, maxX - minX),
        Math.max(100, height - 100) / Math.max(240, maxY - minY),
      ),
    );
    return { x: (minX + maxX) / 2, y: (minY + maxY) / 2, zoom };
  }
}
