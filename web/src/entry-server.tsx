import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { App } from "./App";
import type { Locale } from "./seo";

export function renderPage(locale: Locale) {
  const client = new QueryClient();
  try {
    return renderToString(
      <QueryClientProvider client={client}>
        <App initialLocale={locale} />
      </QueryClientProvider>,
    );
  } finally {
    client.clear();
  }
}
