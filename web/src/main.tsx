import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { App } from "./App";
import "./styles.css";
import "./native-responsive.css";
import "./landing-compact.css";
const root = createRootRoute();
const route = createRoute({
  getParentRoute: () => root,
  path: "/",
  component: App,
});
const englishRoute = createRoute({
  getParentRoute: () => root,
  path: "/en",
  component: App,
});
const router = createRouter({
  routeTree: root.addChildren([route, englishRoute]),
  basepath: import.meta.env.BASE_URL,
});
const client = new QueryClient();
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </React.StrictMode>,
);
