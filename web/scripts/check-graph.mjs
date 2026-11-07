import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import ts from "typescript";

// Exercise the pure solver without installing a browser or a second test runtime.
const sourceUrl = new URL("../src/graph-simulation.ts", import.meta.url);
const source = readFileSync(sourceUrl, "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2022,
    esModuleInterop: true,
  },
}).outputText;
const module = { exports: {} };
new Function("module", "exports", "require", compiled)(
  module,
  module.exports,
  createRequire(sourceUrl),
);
const { GraphSimulation, ALPHA_MIN } = module.exports;
const fixtures = JSON.parse(
  readFileSync(new URL("../src/fixtures.json", import.meta.url), "utf8"),
);
const projects = fixtures.projects.filter((p) => p.status !== "Archived");
const settled = () => {
  const sim = new GraphSimulation();
  sim.sync(projects);
  sim.settle();
  return sim;
};

const sim = settled();
assert.ok(sim.alpha <= ALPHA_MIN);
assert.equal(
  sim.nodes.length,
  projects.length + new Set(projects.map((p) => p.domain)).size,
);
assert.ok(
  sim.nodes.every((n) => [n.x, n.y, n.vx, n.vy].every(Number.isFinite)),
);
let nearestOwn = 0;
for (let i = 0; i < sim.nodes.length; i++) {
  const a = sim.nodes[i];
  for (const b of sim.nodes.slice(i + 1))
    assert.ok(
      Math.hypot(a.x - b.x, a.y - b.y) >= (a.radius + b.radius) * 0.85,
      `Overlapping ${a.key} and ${b.key}`,
    );
  if (a.projectId !== undefined) {
    const hubs = sim.nodes
      .filter((n) => n.projectId === undefined)
      .sort(
        (x, y) =>
          Math.hypot(x.x - a.x, x.y - a.y) - Math.hypot(y.x - a.x, y.y - a.y),
      );
    if (hubs[0].domain === a.domain) nearestOwn++;
  }
}
assert.ok(
  nearestOwn / projects.length >= 0.9,
  `${nearestOwn}/${projects.length} in their own domain`,
);

const before = new Map(sim.nodes.map((n) => [n.key, { x: n.x, y: n.y }]));
assert.equal(
  sim.sync([...projects].reverse()),
  false,
  "A UI sort does not rebuild the graph",
);
const fewer = projects.slice(1);
assert.equal(sim.sync(fewer), true);
assert.ok(sim.alpha >= 0.6);
for (const n of sim.nodes)
  assert.deepEqual(
    { x: n.x, y: n.y },
    before.get(n.key),
    "Filter preserves surviving positions",
  );
assert.ok(
  sim.nodes
    .filter((n) => n.projectId !== undefined)
    .every((n) => fewer[n.index].id === n.projectId),
);

const at30 = new GraphSimulation(),
  at144 = new GraphSimulation();
at30.sync(projects);
at144.sync(projects);
for (let i = 0; i < 60; i++) at30.step(1 / 30);
for (let i = 0; i < 288; i++) at144.step(1 / 144);
for (let i = 0; i < at30.nodes.length; i++) {
  assert.ok(Math.abs(at30.nodes[i].x - at144.nodes[i].x) < 1e-9);
  assert.ok(Math.abs(at30.nodes[i].y - at144.nodes[i].y) < 1e-9);
}

const drag = settled();
const node = drag.nodes.find((n) => n.projectId !== undefined);
const neighborIndex = drag.neighbors[drag.nodes.indexOf(node)][0];
const neighborBefore = { ...drag.nodes[neighborIndex] };
drag.pin(node.key, node.x + 110, node.y + 80);
const pinned = { x: node.x, y: node.y };
for (let i = 0; i < 30; i++) drag.tick();
assert.deepEqual(
  { x: node.x, y: node.y },
  pinned,
  "Dragging node remains pinned",
);
assert.ok(
  Math.hypot(
    drag.nodes[neighborIndex].x - neighborBefore.x,
    drag.nodes[neighborIndex].y - neighborBefore.y,
  ) > 1,
  "Drag pulls its neighbors",
);
drag.unpin(node.key);
drag.settle();
assert.equal(drag.alphaTarget, 0);
assert.ok(!node.fixed && !drag.warm);
drag.shake();
assert.ok(
  drag.alpha >= 0.8 && drag.nodes.some((n) => Math.hypot(n.vx, n.vy) > 10),
);
drag.settle();
assert.ok(!drag.warm && drag.nodes.every((n) => Number.isFinite(n.x + n.y)));

const relationProjects = structuredClone(projects.slice(0, 5));
const folders = [
  "C:/P/Shop",
  "C:/P/Shop/app",
  "C:/P/Group/A",
  "C:/P/Group/B",
  "C:/P/Other",
];
relationProjects.forEach((p, i) => {
  p.folders = [folders[i]];
  p.tags = [];
});
const context = structuredClone(fixtures);
context.config.roots = [{ path: "C:/P" }];
const relations = new GraphSimulation(context);
relations.sync(relationProjects);
const family = (a, b) =>
  relations.edges.some(
    (e) =>
      e.kind === "folder" &&
      [relations.nodes[e.a].projectId, relations.nodes[e.b].projectId]
        .sort()
        .join() ===
        [relationProjects[a].id, relationProjects[b].id].sort().join(),
  );
assert.ok(family(0, 1), "Nested project belongs to its owner");
assert.ok(family(2, 3), "Sibling projects grouped");
assert.ok(!family(0, 4), "Source root does not group unrelated projects");
assert.ok(relations.edges.every((e) => e.a !== e.b));
assert.ok(
  relations.edges
    .filter((e) => e.kind === "stack")
    .every((e) => relations.nodes[e.a].domain !== relations.nodes[e.b].domain),
);
const withTags = structuredClone(relationProjects.slice(0, 2));
const domainConcept = fixtures.ontology.concepts.find(
  (c) => c.name === fixtures.domains[1].name,
);
withTags[0].tags = [
  { concept: domainConcept.id, weight: 0.8 },
  { concept: 10, weight: 0.8 },
  { concept: 11, weight: 0.9 },
];
withTags[1].tags = [
  { concept: 10, weight: 0.8 },
  { concept: 11, weight: 0.9 },
];
withTags[0].folders = ["C:/P/A"];
withTags[1].folders = ["C:/P/B"];
withTags[0].stack = [];
withTags[1].stack = [];
const tagged = new GraphSimulation(context);
tagged.sync(withTags);
assert.ok(
  tagged.edges.some((e) => e.kind === "alsoDomain"),
  "Weighted ontology tag adds secondary domain hub",
);
assert.ok(
  tagged.edges.some((e) => e.kind === "tags"),
  "Two shared non-domain concepts add a weak relation",
);
const fullFit = drag.fit(1000, 700),
  domainFit = drag.fit(1000, 700, 0);
assert.ok(
  fullFit.zoom >= 0.25 && fullFit.zoom <= 1.6 && domainFit.zoom >= fullFit.zoom,
);
console.log(
  `Graph checks passed: ${projects.length} projects, ${nearestOwn} nearest their own domain; collisions, stable filtering, 30/144Hz equivalence, drag propagation, shake/settle, family links, camera fit.`,
);

// Load only the local demo data modules, using the same TypeScript compiler as
// the solver checks. No browser, server, package installation or generated files.
const localModules = new Map();
function loadLocalTypeScript(url) {
  if (localModules.has(url.href)) return localModules.get(url.href).exports;
  const local = { exports: {} };
  localModules.set(url.href, local);
  const javascript = ts.transpileModule(readFileSync(url, "utf8"), {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2022,
      esModuleInterop: true,
    },
  }).outputText;
  new Function("module", "exports", "require", javascript)(
    local,
    local.exports,
    (specifier) => {
      assert.ok(
        specifier.startsWith("./"),
        `Unexpected fixture dependency ${specifier}`,
      );
      if (specifier.endsWith(".json"))
        return JSON.parse(readFileSync(new URL(specifier, url), "utf8"));
      return loadLocalTypeScript(new URL(`${specifier}.ts`, url));
    },
  );
  return local.exports;
}
const expandedProjects = loadLocalTypeScript(
  new URL("../src/data.ts", import.meta.url),
).projects;
assert.equal(expandedProjects.length, 152);
assert.equal(
  new Set(expandedProjects.map((p) => p.id)).size,
  expandedProjects.length,
);
assert.equal(
  new Set(expandedProjects.map((p) => p.name)).size,
  expandedProjects.length,
);
assert.deepEqual(
  expandedProjects
    .slice(0, fixtures.projects.length)
    .map(({ packages, ...p }) => p),
  fixtures.projects,
  "Native fixture records are preserved",
);
const managers = new Set(
  expandedProjects.flatMap((p) => p.packages.map((pkg) => pkg.manager)),
);
for (const manager of ["Npm", "Pip", "Cargo", "Composer", "Go", "Docker"])
  assert.ok(managers.has(manager), `${manager} has simulated manifest data`);
const extended = new GraphSimulation();
const expandedVisible = expandedProjects.filter((p) => p.status !== "Archived");
assert.equal(expandedVisible.length, 150);
extended.sync(expandedVisible);
const crossDomain = extended.edges.filter(
  (edge) => extended.nodes[edge.a].domain !== extended.nodes[edge.b].domain,
).length;
assert.ok(
  crossDomain >= 150,
  "Expanded workspace has substantial cross-domain relations",
);
const edgeCounts = {};
for (const edge of extended.edges)
  edgeCounts[edge.kind] = (edgeCounts[edge.kind] ?? 0) + 1;
assert.ok(
  edgeCounts.folder >= 70 &&
    edgeCounts.alsoDomain >= 70 &&
    edgeCounts.stack >= 20,
);
const solveStart = performance.now();
extended.settle();
const solveMs = performance.now() - solveStart;
assert.ok(!extended.warm);
assert.ok(
  extended.nodes.every((n) => [n.x, n.y, n.vx, n.vy].every(Number.isFinite)),
);
for (let i = 0; i < extended.nodes.length; i++) {
  const a = extended.nodes[i];
  for (const b of extended.nodes.slice(i + 1))
    assert.ok(
      Math.hypot(a.x - b.x, a.y - b.y) >= (a.radius + b.radius) * 0.7,
      `Expanded overlap ${a.key}/${b.key}`,
    );
}
console.log(
  `Expanded demo passed: ${expandedProjects.length} projects, ${expandedVisible.length} visible, ${extended.edges.length} edges (${crossDomain} cross-domain), ${JSON.stringify(edgeCounts)}, settled in ${solveMs.toFixed(1)}ms.`,
);
