export type Locale = "fr" | "en";

export const author = "Djidjelli Youcef";
export const repositoryUrl = "https://github.com/djdevpro/rustproject";
// Planned GitHub Pages address, overridable before publication.
export const defaultSiteUrl = "https://djdevpro.github.io/rustproject/";

export const seoCopy = {
  fr: {
    title: "Rust Project | Gérez vos projets locaux",
    description:
      "Retrouvez et organisez vos projets locaux avec Rust Project. Recherche hybride, graphe, modèles locaux et serveur MCP. Une application de bureau open source.",
    imageAlt: "Rust Project et sa bibliothèque de projets dans la démo web",
    locale: "fr_FR",
  },
  en: {
    title: "Rust Project | Organize your local projects",
    description:
      "Find and organize your local projects with Rust Project. Hybrid search, a project graph, local models and an MCP server. An open source desktop app.",
    imageAlt: "Rust Project and its project library in the web demo",
    locale: "en_US",
  },
} as const;

export function normalizeSiteUrl(value = defaultSiteUrl) {
  const url = new URL(value);
  if (!/^https?:$/.test(url.protocol) || url.username || url.password)
    throw new Error("VITE_SITE_URL must be an HTTP(S) URL without credentials");
  url.search = "";
  url.hash = "";
  if (!url.pathname.endsWith("/")) url.pathname += "/";
  return url.href;
}

export function localePath(lang: Locale, base = "/") {
  return `${base.endsWith("/") ? base : base + "/"}${lang === "en" ? "en/" : ""}`;
}

export function localeFromPath(path: string): Locale {
  return /\/en(?:\/|\/index\.html)?$/.test(path) ? "en" : "fr";
}

export function escapeHtml(value: string) {
  return value.replace(
    /[&<>"']/g,
    (char) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[char]!,
  );
}

export function renderSeoHead(
  lang: Locale,
  siteUrl: string,
  assetBase: string,
) {
  const base = normalizeSiteUrl(siteUrl);
  const copy = seoCopy[lang];
  const pageUrl = new URL(lang === "en" ? "en/" : "", base).href;
  const imageUrl = new URL("social-card.png", base).href;
  const person = { "@type": "Person", name: author };
  const structuredData = {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "WebSite",
        "@id": `${base}#website`,
        name: "Rust Project",
        url: base,
        inLanguage: ["fr", "en"],
        author: person,
      },
      {
        "@type": "WebPage",
        "@id": `${pageUrl}#webpage`,
        url: pageUrl,
        name: copy.title,
        description: copy.description,
        inLanguage: lang,
        isPartOf: { "@id": `${base}#website` },
        mainEntity: { "@id": `${base}#software` },
        author: person,
      },
      {
        "@type": "SoftwareApplication",
        "@id": `${base}#software`,
        name: "Rust Project",
        url: base,
        applicationCategory: "DeveloperApplication",
        description: copy.description,
        author: person,
        creator: person,
        downloadUrl: `${repositoryUrl}/releases`,
        sameAs: repositoryUrl,
        image: imageUrl,
        license: [
          `${repositoryUrl}/blob/main/LICENSE-MIT`,
          `${repositoryUrl}/blob/main/LICENSE-APACHE`,
        ],
      },
    ],
  };
  const meta = (name: string, content: string, property = false) =>
    `<meta ${property ? "property" : "name"}="${name}" content="${escapeHtml(content)}" />`;
  return [
    `<title>${escapeHtml(copy.title)}</title>`,
    meta("description", copy.description),
    meta("author", author),
    meta("robots", "index, follow, max-image-preview:large"),
    meta("theme-color", "#121212"),
    meta("color-scheme", "dark"),
    `<link rel="icon" type="image/svg+xml" href="${escapeHtml(assetBase)}icon.svg" />`,
    `<link rel="canonical" href="${escapeHtml(pageUrl)}" />`,
    ...(["fr", "en", "x-default"] as const).map(
      (language) =>
        `<link rel="alternate" hreflang="${language}" href="${escapeHtml(new URL(language === "en" ? "en/" : "", base).href)}" />`,
    ),
    meta("og:type", "website", true),
    meta("og:site_name", "Rust Project", true),
    meta("og:title", copy.title, true),
    meta("og:description", copy.description, true),
    meta("og:url", pageUrl, true),
    meta("og:locale", copy.locale, true),
    meta(
      "og:locale:alternate",
      seoCopy[lang === "fr" ? "en" : "fr"].locale,
      true,
    ),
    meta("og:image", imageUrl, true),
    meta("og:image:secure_url", imageUrl, true),
    meta("og:image:type", "image/png", true),
    meta("og:image:width", "1200", true),
    meta("og:image:height", "630", true),
    meta("og:image:alt", copy.imageAlt, true),
    meta("twitter:card", "summary_large_image"),
    meta("twitter:title", copy.title),
    meta("twitter:description", copy.description),
    meta("twitter:image", imageUrl),
    meta("twitter:image:alt", copy.imageAlt),
    `<script type="application/ld+json">${JSON.stringify(structuredData).replace(/</g, "\\u003c")}</script>`,
  ].join("\n    ");
}

export function applySeoHtml(
  html: string,
  lang: Locale,
  siteUrl: string,
  assetBase: string,
) {
  return html
    .replace(/<html lang="[^"]*"/, `<html lang="${lang}"`)
    .replace(
      /<!--seo:start-->[\s\S]*?<!--seo:end-->/,
      `<!--seo:start-->\n    ${renderSeoHead(lang, siteUrl, assetBase)}\n    <!--seo:end-->`,
    );
}
