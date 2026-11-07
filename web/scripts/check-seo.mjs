import assert from "node:assert/strict";
import { readFile, access } from "node:fs/promises";
import { loadEnv } from "vite";

const configured = loadEnv("production", process.cwd(), "VITE_").VITE_SITE_URL;
const site = new URL(configured || "https://djdevpro.github.io/rustproject/");
site.search = "";
site.hash = "";
if (!site.pathname.endsWith("/")) site.pathname += "/";
const author = "Djidjelli Youcef";
for (const locale of ["fr", "en"]) {
  const file = locale === "fr" ? "dist/index.html" : "dist/en/index.html";
  const html = await readFile(file, "utf8");
  const head = html.match(/<head>([\s\S]*?)<\/head>/)?.[1];
  assert.ok(head, `${file} has a head`);
  const meta = (key) =>
    head.match(
      new RegExp(`<meta (?:name|property)="${key}" content="([^"]*)"`),
    )?.[1];
  const canonical = new URL(locale === "en" ? "en/" : "", site).href;
  assert.ok(html.includes(`<html lang="${locale}"`), `${file} language`);
  assert.equal(
    (head.match(/<title>/g) || []).length,
    1,
    `${file} unique title`,
  );
  assert.equal(
    (html.match(/<h1>/g) || []).length,
    1,
    `${file} unique main heading`,
  );
  assert.equal(meta("author"), author, `${file} author`);
  assert.ok(meta("description")?.length > 100, `${file} description`);
  assert.ok(
    head.includes(`<link rel="canonical" href="${canonical}"`),
    `${file} canonical`,
  );
  assert.equal(meta("og:url"), canonical, `${file} sharing URL`);
  assert.equal(
    meta("twitter:card"),
    "summary_large_image",
    `${file} sharing card`,
  );
  assert.equal(meta("og:image"), new URL("social-card.png", site).href);
  assert.equal(meta("og:locale"), locale === "fr" ? "fr_FR" : "en_US");
  assert.equal(meta("theme-color"), "#121212");
  for (const alternate of ["fr", "en", "x-default"]) {
    const url = new URL(alternate === "en" ? "en/" : "", site).href;
    assert.ok(
      head.includes(`hreflang="${alternate}" href="${url}"`),
      `${file} alternate ${alternate}`,
    );
  }
  const schema = JSON.parse(
    head.match(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/)[1],
  );
  const application = schema["@graph"].find(
    (item) => item["@type"] === "SoftwareApplication",
  );
  assert.equal(application.author.name, author);
  assert.equal(application.creator.name, author);
  assert.ok(!application.aggregateRating, "No fabricated ratings");
  assert.ok(
    html.includes(
      locale === "fr" ? "Vos projets, réunis." : "Your projects, together.",
    ),
    `${file} pre-rendered heading`,
  );
  assert.ok(
    html.includes(
      locale === "fr" ? "Découverte des projets" : "Project discovery",
    ),
    `${file} pre-rendered content`,
  );
  assert.ok(!/noindex/.test(head), `${file} is indexable`);
  for (const match of html.matchAll(/(?:src|href)="([^"#]+)"/g)) {
    if (!match[1].startsWith("/") || match[1].startsWith("//")) continue;
    assert.ok(
      match[1].startsWith(site.pathname),
      `${file} respects the asset base for ${match[1]}`,
    );
    if (match[1] === site.pathname) continue;
    const relative = match[1].slice(site.pathname.length).split("#")[0];
    if (!relative || relative.endsWith("/")) continue;
    await access(`dist/${relative}`);
  }
}
const sitemap = await readFile("dist/sitemap.xml", "utf8");
for (const url of [site.href, new URL("en/", site).href])
  assert.ok(sitemap.includes(`<loc>${url}</loc>`), `Sitemap lists ${url}`);
assert.ok(
  (await readFile("dist/robots.txt", "utf8")).includes(
    `Sitemap: ${site.href}sitemap.xml`,
  ),
);
const png = await readFile("dist/social-card.png");
assert.equal(png.subarray(1, 4).toString(), "PNG");
assert.equal(png.readUInt32BE(16), 1200);
assert.equal(png.readUInt32BE(20), 630);
console.log(
  "SEO verified for FR and EN with static content, author, metadata, schema, assets and 1200x630 sharing image.",
);
