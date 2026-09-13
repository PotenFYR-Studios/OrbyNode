import { describe, expect, test } from "bun:test";
import {
  applyMeta,
  canonicalUrl,
  escapeAttr,
  jsonLdFor,
  outputPathFor,
  seoWeight,
  sitemapXml,
} from "./seo";

const SHELL = `<!doctype html>
<html lang="en">
  <head>
    <title>OrbyNode — Persistent Control Plane for Coding Agents</title>
    <meta name="description" content="Run persistent coding agents." />
    <meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1" />
    <link rel="canonical" href="https://orbynode.docs.potenfyr.in/" />
    <meta property="og:title" content="OrbyNode — Persistent Control Plane for Coding Agents" />
    <meta property="og:url" content="https://orbynode.docs.potenfyr.in/" />
    <meta name="twitter:title" content="OrbyNode — Persistent Control Plane for Coding Agents" />
  </head>
  <body><div id="root"></div></body>
</html>`;

describe("canonicalUrl", () => {
  test("root stays a bare trailing slash", () => {
    expect(canonicalUrl("https://x.example", "/")).toBe("https://x.example/");
  });
  test("directory routes get a trailing slash", () => {
    expect(canonicalUrl("https://x.example", "/docs/installation")).toBe(
      "https://x.example/docs/installation/",
    );
  });
  test("strips trailing slashes before normalizing", () => {
    expect(canonicalUrl("https://x.example", "/docs/")).toBe("https://x.example/docs/");
  });
  test("siteUrl trailing slash is not doubled", () => {
    expect(canonicalUrl("https://x.example/", "/about")).toBe("https://x.example/about/");
  });
});

describe("outputPathFor", () => {
  test("root -> index.html", () => {
    expect(outputPathFor("/")).toBe("index.html");
  });
  test("nested route -> directory index.html", () => {
    expect(outputPathFor("/docs/installation")).toBe("docs/installation/index.html");
  });
  test("trailing slash tolerated", () => {
    expect(outputPathFor("/about/")).toBe("about/index.html");
  });
});

describe("escapeAttr", () => {
  test("escapes quotes, ampersands, angle brackets", () => {
    expect(escapeAttr(`a & "b" <c>`)).toBe("a &amp; &quot;b&quot; &lt;c&gt;");
  });
});

describe("applyMeta", () => {
  const patch = {
    title: 'Guide: "Quotes" & <Angles> $& $1',
    description: "Uses markdown-ish $& sequences & more",
    canonical: "https://x.example/docs/guide/",
  };

  test("replaces every SEO field without breaking on $-patterns", () => {
    const out = applyMeta(SHELL, patch);
    expect(out).toContain(`<title>Guide: &quot;Quotes&quot; &amp; &lt;Angles&gt; $&amp; $1</title>`);
    expect(out).toContain(`content="Uses markdown-ish $&amp; sequences &amp; more"`);
    expect(out).toContain(`href="https://x.example/docs/guide/"`);
    expect(out).toContain(`property="og:url" content="https://x.example/docs/guide/"`);
    expect(out).toContain(`name="twitter:title" content="Guide:`);
    // exactly one of each
    expect(out.match(/<title>/g)).toHaveLength(1);
    expect(out.match(/rel="canonical"/g)).toHaveLength(1);
  });

  test("injects JSON-LD before </head>", () => {
    const out = applyMeta(SHELL, { ...patch, jsonLd: '<script type="application/ld+json">{}</script>' });
    expect(out).toContain('<script type="application/ld+json">{}</script>\n</head>');
  });

  test("can override robots (404 noindex)", () => {
    const out = applyMeta(SHELL, { ...patch, robots: "noindex" });
    expect(out).toContain('name="robots" content="noindex"');
  });

  test("untouched shell passes through for empty patch fields", () => {
    const out = applyMeta(SHELL, { title: "T", description: "D", canonical: "https://x.example/" });
    expect(out).toContain('name="robots" content="index, follow');
  });
});

describe("jsonLdFor", () => {
  test("home emits WebSite + SoftwareApplication", () => {
    const out = jsonLdFor({
      siteUrl: "https://x.example",
      route: "/",
      seo: { title: "Home", description: "d" },
    });
    expect(out).toContain('"@type":"WebSite"');
    expect(out).toContain('"@type":"SoftwareApplication"');
  });
  test("docs index emits CollectionPage", () => {
    const out = jsonLdFor({
      siteUrl: "https://x.example",
      route: "/docs",
      seo: { title: "Docs", description: "d" },
    });
    expect(out).toContain('"@type":"CollectionPage"');
  });
  test("guide emits TechArticle + BreadcrumbList with trailing-slash URLs", () => {
    const out = jsonLdFor({
      siteUrl: "https://x.example",
      route: "/docs/installation",
      seo: { title: "Install | OrbyNode", description: "d" },
      doc: { title: "Installation", description: "d", kind: "guide" },
    });
    expect(out).toContain('"@type":"TechArticle"');
    expect(out).toContain('"@type":"BreadcrumbList"');
    expect(out).toContain("https://x.example/docs/installation/");
    expect(out).not.toContain("architecture-decisions");
  });
  test("unknown route yields empty string", () => {
    expect(
      jsonLdFor({ siteUrl: "https://x.example", route: "/nowhere", seo: { title: "t", description: "d" } }),
    ).toBe("");
  });
});

describe("sitemapXml", () => {
  test("canonical trailing-slash URLs, correct ordering", () => {
    const xml = sitemapXml("https://x.example", ["/", "/docs", "/docs/a"], (r) => seoWeight(r, "guide"));
    expect(xml).toContain("<loc>https://x.example/</loc>");
    expect(xml).toContain("<loc>https://x.example/docs/</loc>");
    expect(xml).toContain("<loc>https://x.example/docs/a/</loc>");
    expect(xml).toContain("<priority>1.0</priority>");
    expect(xml.startsWith('<?xml version="1.0" encoding="UTF-8"?>')).toBe(true);
  });
});
