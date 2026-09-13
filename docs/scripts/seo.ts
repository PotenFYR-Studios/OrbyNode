// Pure SEO helpers shared by the prerender step and its tests. No I/O,
// no Vite imports - everything here must stay trivially testable.

export interface PageSeo {
  title: string;
  description: string;
}

/** Canonical URL for a route: custom-domain root, trailing directory slash. */
export function canonicalUrl(siteUrl: string, route: string): string {
  const clean = route.replace(/\/+$/, "") || "/";
  const path = clean === "/" ? "/" : `${clean}/`;
  return `${siteUrl.replace(/\/+$/, "")}${path}`;
}

/** dist-relative output path: trailing-directory index.html per route. */
export function outputPathFor(route: string): string {
  const clean = route.replace(/\/+$/, "");
  if (clean === "" || clean === "/") return "index.html";
  return `${clean.replace(/^\//, "")}/index.html`;
}

/** Escape a string for safe use inside a double-quoted HTML attribute. */
export function escapeAttr(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export interface MetaPatch extends PageSeo {
  canonical: string;
  /** og:image / twitter:image absolute URL. */
  image?: string;
  /** Override the robots directive (e.g. "noindex" for 404.html). */
  robots?: string;
  /** Pre-rendered JSON-LD block(s) injected before `</head>`. */
  jsonLd?: string;
  /** Pre-rendered markup injected into `<div id="root">`. */
  body?: string;
}

/**
 * Apply per-route metadata to the built index.html shell.
 * Every replacement uses a function replacer so content containing
 * `$&`, `$1`, or backslashes is inserted verbatim.
 */
export function applyMeta(shell: string, patch: MetaPatch): string {
  const esc = escapeAttr;
  let out = shell;

  const swap = (selector: RegExp, build: () => string): void => {
    out = out.replace(selector, () => build());
  };

  swap(/<title>[\s\S]*?<\/title>/, () => `<title>${esc(patch.title)}</title>`);
  swap(
    /<meta\s+name="description"\s+content="[^"]*"\s*\/?>/,
    () => `<meta name="description" content="${esc(patch.description)}" />`,
  );
  swap(
    /<link\s+rel="canonical"\s+href="[^"]*"\s*\/?>/,
    () => `<link rel="canonical" href="${esc(patch.canonical)}" />`,
  );
  if (patch.robots) {
    const robots = patch.robots;
    swap(
      /<meta\s+name="robots"\s+content="[^"]*"\s*\/?>/,
      () => `<meta name="robots" content="${esc(robots)}" />`,
    );
  }
  swap(
    /<meta\s+property="og:title"\s+content="[^"]*"\s*\/?>/,
    () => `<meta property="og:title" content="${esc(patch.title)}" />`,
  );
  swap(
    /<meta\s+property="og:description"\s+content="[^"]*"\s*\/?>/,
    () => `<meta property="og:description" content="${esc(patch.description)}" />`,
  );
  swap(
    /<meta\s+property="og:url"\s+content="[^"]*"\s*\/?>/,
    () => `<meta property="og:url" content="${esc(patch.canonical)}" />`,
  );
  if (patch.image) {
    swap(
      /<meta\s+property="og:image"\s+content="[^"]*"\s*\/?>/,
      () => `<meta property="og:image" content="${esc(patch.image!)}" />`,
    );
    swap(
      /<meta\s+name="twitter:image"\s+content="[^"]*"\s*\/?>/,
      () => `<meta name="twitter:image" content="${esc(patch.image!)}" />`,
    );
  }
  swap(
    /<meta\s+name="twitter:title"\s+content="[^"]*"\s*\/?>/,
    () => `<meta name="twitter:title" content="${esc(patch.title)}" />`,
  );
  swap(
    /<meta\s+name="twitter:description"\s+content="[^"]*"\s*\/?>/,
    () => `<meta name="twitter:description" content="${esc(patch.description)}" />`,
  );

  if (patch.jsonLd && patch.jsonLd.trim().length > 0) {
    out = out.replace("</head>", () => `${patch.jsonLd}\n</head>`);
  }
  if (typeof patch.body === "string") {
    out = out.replace('<div id="root"></div>', () => `<div id="root">${patch.body}</div>`);
  }
  return out;
}

// JSON-LD -------------------------------------------------------------------

export function jsonLdScript(data: unknown): string {
  return `<script type="application/ld+json">\n${JSON.stringify(data)}\n</script>`;
}

export interface JsonLdInputs {
  siteUrl: string;
  route: string;
  seo: PageSeo;
  /** Set for doc routes: drives TechArticle + BreadcrumbList. */
  doc?: { title: string; description: string; kind: "guide" | "adr" };
}

const ORG = { "@type": "Organization", name: "PotenFYR Studios" };

/** JSON-LD block(s) for a route, joined by newlines. Empty for unknown routes. */
export function jsonLdFor(inputs: JsonLdInputs): string {
  const { siteUrl, route, seo } = inputs;
  const clean = route.replace(/\/+$/, "") || "/";
  const url = canonicalUrl(siteUrl, clean);

  if (clean === "/") {
    return [
      jsonLdScript({
        "@context": "https://schema.org",
        "@type": "WebSite",
        name: "OrbyNode",
        url,
        description: seo.description,
        publisher: ORG,
        inLanguage: "en",
      }),
      jsonLdScript({
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        name: "OrbyNode",
        applicationCategory: "DeveloperApplication",
        operatingSystem: "macOS, Windows, Linux",
        url,
        description: seo.description,
        author: ORG,
        offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
        license: "https://spdx.org/licenses/Apache-2.0.html",
      }),
    ].join("\n");
  }
  if (clean === "/docs") {
    return jsonLdScript({
      "@context": "https://schema.org",
      "@type": "CollectionPage",
      name: "OrbyNode Documentation",
      url,
      description: seo.description,
      author: ORG,
    });
  }
  if (clean === "/about") {
    return jsonLdScript({
      "@context": "https://schema.org",
      "@type": "WebPage",
      name: seo.title,
      url,
      description: seo.description,
      author: ORG,
    });
  }
  if (inputs.doc) {
    return [
      jsonLdScript({
        "@context": "https://schema.org",
        "@type": "TechArticle",
        headline: seo.title,
        description: inputs.doc.description,
        url,
        author: ORG,
        publisher: ORG,
      }),
      jsonLdScript({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        itemListElement: [
          { "@type": "ListItem", position: 1, name: "OrbyNode", item: canonicalUrl(siteUrl, "/") },
          { "@type": "ListItem", position: 2, name: "Docs", item: canonicalUrl(siteUrl, "/docs") },
          ...(inputs.doc.kind === "adr"
            ? [{ "@type": "ListItem", position: 3, name: "ADRs", item: canonicalUrl(siteUrl, "/docs") }]
            : []),
          {
            "@type": "ListItem",
            position: inputs.doc.kind === "adr" ? 4 : 3,
            name: inputs.doc.title,
          },
        ],
      }),
    ].join("\n");
  }
  return "";
}

// Sitemap -------------------------------------------------------------------

export interface SitemapEntry {
  changefreq: "daily" | "weekly" | "monthly" | "yearly";
  priority: string;
}

/** Crawl hints per route; doc kind decides weight. */
export function seoWeight(route: string, docKind?: "guide" | "adr"): SitemapEntry {
  const clean = route.replace(/\/+$/, "") || "/";
  if (clean === "/") return { changefreq: "weekly", priority: "1.0" };
  if (clean === "/docs") return { changefreq: "weekly", priority: "0.9" };
  if (docKind === "adr") return { changefreq: "monthly", priority: "0.5" };
  if (docKind === "guide") return { changefreq: "monthly", priority: "0.7" };
  return { changefreq: "monthly", priority: "0.4" };
}

/** Full sitemap.xml document. Canonical (custom-domain, trailing-slash) URLs. */
export function sitemapXml(
  siteUrl: string,
  routes: readonly string[],
  weightOf: (route: string) => SitemapEntry,
): string {
  const urls = routes
    .map((route) => {
      const weight = weightOf(route);
      return [
        "  <url>",
        `    <loc>${escapeAttr(canonicalUrl(siteUrl, route))}</loc>`,
        `    <changefreq>${weight.changefreq}</changefreq>`,
        `    <priority>${weight.priority}</priority>`,
        "  </url>",
      ].join("\n");
    })
    .join("\n");
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    urls,
    "</urlset>",
    "",
  ].join("\n");
}
