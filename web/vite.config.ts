import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { applySeoHtml, localeFromPath, normalizeSiteUrl } from "./src/seo";
export default defineConfig(({ mode, command, isPreview }) => {
  const env = loadEnv(mode, process.cwd(), "VITE_");
  const siteUrl = normalizeSiteUrl(env.VITE_SITE_URL);
  const base =
    command === "serve" && !isPreview ? "/" : new URL(siteUrl).pathname;
  return {
    plugins: [
      react(),
      tailwindcss(),
      {
        name: "rust-project-seo",
        transformIndexHtml(html, context) {
          const path = (context.originalUrl ?? context.path).split(/[?#]/)[0];
          return applySeoHtml(html, localeFromPath(path), siteUrl, base);
        },
      },
    ],
    base,
    build: {
      rollupOptions: {
        output: {
          manualChunks: {
            three: ["three"],
            runtime: [
              "react",
              "react-dom",
              "@tanstack/react-router",
              "@tanstack/react-query",
            ],
          },
        },
      },
    },
  };
});
