// Multi-page emit: after `vite build`, render every route to a real HTML
// file so direct refreshes, crawlers and no-JS visitors get full content
// plus per-route title, description, canonical, OG/Twitter tags and
// JSON-LD. Sitemap is generated from the same route table so it can never
// drift from what is actually emitted.
//
//   /                    -> dist/index.html
//   /about               -> dist/about/index.html
//   /docs                -> dist/docs/index.html
//   /docs/<slug>         -> dist/docs/<slug>/index.html        (25 guides)
//   /docs/adr/<slug>     -> dist/docs/adr/<slug>/index.html    (22 ADRs)
//
// Canonical URLs always carry the trailing directory slash: GitHub Pages
// 301s the bare path to the directory form, so that is the canonical
// target. They also always use the custom domain, even when the build
// runs under a /OrbyNode/ base - the site is served at the domain root.
import { createServer } from "vite";
import { renderToString } from "react-dom/server";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { applyMeta, jsonLdFor, outputPathFor, seoWeight, sitemapXml } from "./seo";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");

type DocMeta = {
  slug: string;
  title: string;
  description: string;
  kind: "guide" | "adr";
  route: string;
};

// DOCS/SITE_URL/routeMeta live in src/content.ts, which uses Vite's
// import.meta.glob - that transform only exists when Vite loads the
// module, so it must come through ssrLoadModule, never a direct import.
type ContentModule = {
  SITE_URL: string;
  SITE_ROUTES: readonly string[];
  routeMeta: (path: string) => { title: string; description: string };
  docByRoute: (path: string) => DocMeta | undefined;
};

const vite = await createServer({
  root,
  server: { middlewareMode: true },
  appType: "custom",
  logLevel: "error",
});

try {
  const content = (await vite.ssrLoadModule("/src/content")) as ContentModule;
  const { SITE_URL, SITE_ROUTES, routeMeta, docByRoute } = content;
  if (typeof SITE_URL !== "string" || SITE_ROUTES.length === 0) {
    throw new Error("content module exported no routes; refusing to prerender");
  }

  const { AppShell } = (await vite.ssrLoadModule("/src/App")) as {
    AppShell: React.ComponentType;
  };

  const dist = resolve(root, process.env.VITE_OUT_DIR ?? "dist");
  const shellPath = resolve(dist, "index.html");
  const shell = await readFile(shellPath, "utf8");
  if (!shell.includes('<div id="root"></div>')) {
    throw new Error("root div placeholder not found in dist/index.html");
  }

  const base = (process.env.VITE_BASE ?? "/").replace(/\/+$/, "");
  const image = `${SITE_URL}/og.svg`;
  let rendered = 0;

  for (const route of SITE_ROUTES) {
    const body = renderToString(
      React.createElement(
        React.StrictMode,
        null,
        React.createElement(
          MemoryRouter,
          { initialEntries: [route], basename: base || undefined },
          React.createElement(AppShell),
        ),
      ),
    );
    if (body.length < 200) {
      throw new Error(`route ${route} rendered suspiciously little markup (${body.length} bytes)`);
    }

    const seo = routeMeta(route);
    const doc = docByRoute(route);
    const html = applyMeta(shell, {
      title: seo.title,
      description: seo.description,
      canonical: `${SITE_URL}${route === "/" ? "/" : route.replace(/\/+$/, "") + "/"}`,
      image,
      jsonLd: jsonLdFor({ siteUrl: SITE_URL, route, seo, doc }),
    });

    const rel = outputPathFor(route);
    const file = resolve(dist, rel);
    await mkdir(dirname(file), { recursive: true });
    await writeFile(file, html.replace('<div id="root"></div>', () => `<div id="root">${body}</div>`));
    rendered += 1;
    console.log(`[prerender] ${route} -> dist/${rel} (${body.length} bytes)`);
  }

  // 404.html: GitHub Pages' not-found handler. Serves the app shell with
  // the not-found route rendered in and marked noindex.
  const notFoundBody = renderToString(
    React.createElement(
      React.StrictMode,
      null,
      React.createElement(
        MemoryRouter,
        { initialEntries: ["/__not_found__"], basename: base || undefined },
        React.createElement(AppShell),
      ),
    ),
  );
  await writeFile(
    resolve(dist, "404.html"),
    applyMeta(shell, {
      title: "Page not found | OrbyNode",
      description: "This OrbyNode documentation page could not be found.",
      canonical: `${SITE_URL}/404.html`,
      image,
      robots: "noindex, follow",
      jsonLd: "",
    }).replace('<div id="root"></div>', () => `<div id="root">${notFoundBody}</div>`),
  );
  console.log(`[prerender] 404 fallback rendered (${notFoundBody.length} bytes)`);

  // Sitemap from the same route table - it cannot drift from the pages.
  await writeFile(
    resolve(dist, "sitemap.xml"),
    sitemapXml(SITE_URL, SITE_ROUTES, (route) => seoWeight(route, docByRoute(route)?.kind)),
  );
  console.log(`[prerender] sitemap.xml written (${SITE_ROUTES.length} routes)`);

  console.log(`[prerender] ${rendered} routes rendered to ${dist}`);
} finally {
  // The server must close on success and failure alike, or the process
  // hangs the CI job after a partial render.
  await vite.close();
}
