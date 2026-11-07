import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Demo } from "./Demo";
import { I18n } from "./i18n";
import "./styles.css";
import "./native-responsive.css";
import "./landing-compact.css";
import "./banner.css";

// Development-only composition. Vite's production entry remains index.html.
// Capture .repository-banner at its fixed 2000 × 667 size.
function RepositoryBanner() {
  return (
    <main className="repository-banner" aria-label="Rust Project repository banner">
      <div className="banner-copy">
        <div className="banner-brand">
          <img src={`${import.meta.env.BASE_URL}icon.svg`} alt="" />
          <h1>Rust Project</h1>
        </div>
        <p className="banner-tagline">Your projects, together.</p>
        <p className="banner-stack">Rust · Ollama · Ollaya · MCP</p>
      </div>
      <div className="banner-product">
        <div className="banner-window">
          <Demo initialPage="desk" />
        </div>
      </div>
    </main>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={new QueryClient()}>
      <I18n initialLocale="en">
        <RepositoryBanner />
      </I18n>
    </QueryClientProvider>
  </React.StrictMode>,
);
