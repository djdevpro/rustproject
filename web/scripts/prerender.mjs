import { readFile, mkdir, writeFile } from "node:fs/promises";
import { createServer, loadEnv } from "vite";

const server = await createServer({
  mode: "production",
  server: { middlewareMode: true },
  appType: "custom",
  logLevel: "error",
});
try {
  const { renderPage } = await server.ssrLoadModule("/src/entry-server.tsx");
  const { normalizeSiteUrl, applySeoHtml, escapeHtml } =
    await server.ssrLoadModule("/src/seo.ts");
  const siteUrl = normalizeSiteUrl(
    loadEnv("production", process.cwd(), "VITE_").VITE_SITE_URL,
  );
  const base = new URL(siteUrl).pathname;
  const template = await readFile("dist/index.html", "utf8");
  for (const lang of ["fr", "en"]) {
    // SSR uses the dev server's root. Public assets must follow the build base.
    const content = renderPage(lang).replace(
      /\b(src|href)="\/icon.svg"/g,
      `$1="${base}icon.svg"`,
    );
    const html = applySeoHtml(template, lang, siteUrl, base).replace(
      '<div id="root"></div>',
      `<div id="root">${content}</div>`,
    );
    const directory = lang === "en" ? "dist/en" : "dist";
    await mkdir(directory, { recursive: true });
    await writeFile(`${directory}/index.html`, html);
  }
  const pages = [siteUrl, new URL("en/", siteUrl).href];
  await writeFile(
    "dist/sitemap.xml",
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${pages.map((url) => `  <url><loc>${escapeHtml(url)}</loc></url>`).join("\n")}\n</urlset>\n`,
  );
  await writeFile(
    "dist/robots.txt",
    `User-agent: *\nAllow: /\n\nSitemap: ${siteUrl}sitemap.xml\n`,
  );
  await writeFile("dist/.nojekyll", "");
  console.log(
    `Prerendered FR and EN with metadata, sitemap and robots for ${siteUrl}`,
  );
} finally {
  await server.close();
}
