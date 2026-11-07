import type { Project } from "./data";
export function projectLinks(projects: Project[]) {
  const links: { source: number; target: number; kind: "folder" | "stack" }[] =
    [];
  for (let a = 0; a < projects.length; a++)
    for (let b = a + 1; b < projects.length; b++) {
      const source = projects[a],
        target = projects[b];
      const folder = source.folders.some((x) =>
        target.folders.some((y) => {
          const xp = x
              .toLowerCase()
              .replaceAll("\\", "/")
              .split("/")
              .slice(0, -1),
            yp = y.toLowerCase().replaceAll("\\", "/").split("/").slice(0, -1);
          return xp.length > 2 && xp.join("/") === yp.join("/");
        }),
      );
      if (folder)
        links.push({ source: source.id, target: target.id, kind: "folder" });
      else if (
        source.domain !== target.domain &&
        source.stack.some((s) => target.stack.includes(s))
      )
        links.push({ source: source.id, target: target.id, kind: "stack" });
    }
  return links;
}
