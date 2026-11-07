# Rust Project web demo

Static React/Vite landing page and simulated desktop app. Publication is manual.

## Preview

```sh
npm ci
npm run dev -- --port 5187 --strictPort
```

Use the port printed by Vite if another dev server is already running. This review used the existing server at http://127.0.0.1:5174/.

## Build

```sh
npm run check:copy
npm run check:graph
npm run build
npm run check:seo
```

The static output is `dist/`. The publication workflow is manual.

## Publish on GitHub Pages

Commit the web source, its lockfile and `.github/workflows/web-pages.yml`. The local `review/` captures, `node_modules/`, `dist/` and `.env*.local` files are ignored. Keep unrelated working-tree changes out of the publication commit.

In the repository's **Settings > Pages**, select **GitHub Actions** as the publishing source. Then run **Publish web landing** from **Actions**, on `main`. It installs locked dependencies with Node 24, checks copy and graph behavior, builds both languages, validates the generated SEO, and publishes only `web/dist`. Pushing code alone does not publish the site.

The workflow reads the site's actual base URL from GitHub Pages and passes it to `VITE_SITE_URL` for the build and SEO checks. Once the deployment succeeds, verify the French and English pages and use the deployment's URL for the repository's **About > Website** field. Without a custom domain, the expected address is `https://djdevpro.github.io/rustproject/`.

## Metadata and SEO

The author is **Djidjelli Youcef**. Each locale has its own title, description, canonical URL, reciprocal `hreflang` links, Open Graph and Twitter metadata. The JSON-LD describes the website, page and desktop application with the same author. It contains no invented ratings or reviews. `public/social-card.png` is the 1200 × 630 sharing image.

The build pre-renders the existing React landing into French `dist/index.html` and English `dist/en/index.html`. Their content and metadata are readable without executing JavaScript. The interactive demo starts when JavaScript loads. The language selector navigates between the two pages, so switching language resets the simulated session. URL language takes precedence over any previously stored preference.

`VITE_SITE_URL` controls canonical URLs, the sitemap, sharing URLs and the production asset base. The default is `https://djdevpro.github.io/rustproject/`. The publication workflow uses the address returned by GitHub Pages. For another host, set its address in `.env.production.local` before publishing, using `.env.example` as a reference. Rebuild after changing it. Development remains at `/` and `/en/` on localhost.

`dist/sitemap.xml`, `dist/robots.txt` and `dist/.nojekyll` are generated during the build. Crawlers read `robots.txt` only at the origin root. On a GitHub Pages project path, the generated project-level `robots.txt` does not replace `https://djdevpro.github.io/robots.txt`. After publication, submit the actual sitemap URL in Search Console and verify indexing there. Build checks validate the local artifacts, not search engine indexing or ranking.

`npm run check:seo` checks both generated pages, language alternates, the author, structured data, asset paths, sitemap and sharing image dimensions.

## Reference sources

The README banner is composed in `banner.html`, `src/banner.tsx` and `src/banner.css` from the actual React demo. Open `/banner.html` on the dev server to export its fixed 2000 × 667 canvas. It opens My desk inside a wide rectangular window. Its current export is `../docs/banner.png`. The second README image is `../docs/library-demo.webp`, a lossless conversion of the supplied library screenshot. The banner export entry is excluded from the production site.

- `../docs/carte.png`, `../docs/grille.png`, `../docs/bureau.png` define the desktop composition.
- `../src/theme.rs` supplies the dark tokens, Segoe UI typography, 60px topbar, 40px footer, 36px controls and 420px inspector.
- `../src/screens/library.rs`, `../src/screens/desk.rs` and `../src/panel.rs` define views and inspector tabs.
- `src/fixtures.json` preserves the original `../fixtures/demo-en.json`. `src/generated-projects.ts` adds deterministic sample projects and shared relationships. Sample records remain in English; application UI and landing copy switch between French and English.
- `src/native-en.json` comes from `../locales/en.json`. `src/i18n.tsx` adds web-specific translations.
- Landing composition reference supplied by the owner, https://supabase.com/modules/vector. No Supabase assets or claims are reused.

## Behavior and limits

The demo opens on Library / Map with 152 sample projects (150 non-archived). Navigation, grid/list views, archived/attention/domain/package filters, text/fuzzy search, sorting, project details, step checkboxes, notes, manual sample creation and card drag/drop to domain or attention filters work in session memory. Configuration and the inspector Edit tab are intentionally disabled. Expand Demo is removed. The inspector heading has no project icon. Reload restores the fixtures. The page URL determines its language.

The Three.js graph uses the force solver ported from `../src/graph.rs`, with a fixed 60Hz simulation and smoothed rendering. Springs, repulsion, collisions, cooling, shake and pinned-node drag preserve the native behavior. Scene and surviving node positions persist across filters. Camera fit, logarithmic zoom easing, cursor-anchored zoom, fading neighbors, twin link particles and label priority follow `../src/screens/map.rs`. Domain focus keeps the other domains in context. Secondary links remain faintly visible at rest for this larger public demo. Pause and reduced-motion settings are respected; offscreen rendering stops and GPU resources are disposed on unmount. The web demo does not implement dragging a graph node out into the sidebar.

Ollama, Ollaya, MCP, filesystem scans, manifest packages and launchers are simulated. No local service is contacted. The web search does not compute real embeddings. Inside the laptop frame, a CSS transform uniformly scales the desktop interface and its text, with the scaled height reserved in the page layout. This avoids iOS WebKit text inflation caused by CSS zoom. The graph updates its pixel density when the frame resizes. Neutral surfaces match the landing; domain colors and amber accents remain.

## Stack

React, TypeScript, TanStack Router and Query, Tailwind CSS, shadcn-style Button (Radix Slot + CVA), Lucide, Three.js. No runtime CDN or external fonts are required.
