import { describe, expect, test } from "bun:test";
import { readdirSync } from "node:fs";
import { resolve } from "node:path";
import {
  ADR_DOCS,
  DOCS,
  SITE_ROUTES,
  docByRoute,
  normalizeMarkdownHref,
  routeMeta,
  validateManifest,
} from "./manifest";

const docsRoot = resolve(import.meta.dir, "..");

describe("documentation manifest", () => {
  test("has unique sources, routes, and kind-scoped slugs", () => {
    expect(() => validateManifest(DOCS)).not.toThrow();
    expect(new Set(DOCS.map((doc) => doc.source)).size).toBe(DOCS.length);
    expect(new Set(DOCS.map((doc) => doc.route)).size).toBe(DOCS.length);
    expect(new Set(DOCS.map((doc) => `${doc.kind}:${doc.slug}`)).size).toBe(DOCS.length);
  });

  test("accounts for every public Markdown source", () => {
    const topLevel = readdirSync(docsRoot)
      .filter((name) => name.endsWith(".md") && name !== "README.md")
      .map((name) => `docs/${name}`);
    const adrs = readdirSync(resolve(docsRoot, "adr"))
      .filter((name) => name.endsWith(".md"))
      .map((name) => `docs/adr/${name}`);
    const roots = ["ARCHITECTURE.md", "ROADMAP.md", "SECURITY.md", "CONTRIBUTING.md"];

    expect(DOCS.map((doc) => doc.source).sort()).toEqual(
      [...topLevel, ...adrs, ...roots].sort(),
    );
    expect(DOCS.some((doc) => doc.source === "Plan.md")).toBe(false);
  });

  test("uses full ADR filenames and stable nested routes", () => {
    expect(ADR_DOCS.length).toBe(22);
    expect(docByRoute("/docs/adr/011-api-interface-stability")?.source).toBe(
      "docs/adr/011-api-interface-stability.md",
    );
    expect(docByRoute("/docs/adr/011-platform-interfaces")?.source).toBe(
      "docs/adr/011-platform-interfaces.md",
    );
    expect(SITE_ROUTES).toContain("/docs/adr/021-herdr-parity-web-workspaces");
  });

  test("returns useful route metadata", () => {
    expect(routeMeta("/docs/installation").title).toBe(
      "Installation | OrbyNode Documentation",
    );
    expect(routeMeta("/docs").title).toBe("Documentation | OrbyNode");
    expect(routeMeta("/missing").title).toBe("Page not found | OrbyNode");
  });
});

describe("Markdown links", () => {
  test("maps docs, root files, ADRs, and anchors to site routes", () => {
    expect(normalizeMarkdownHref("development.md", "docs/getting-started.md")).toBe(
      "/docs/development",
    );
    expect(normalizeMarkdownHref("../ARCHITECTURE.md#runtime", "docs/frontend.md")).toBe(
      "/docs/architecture#runtime",
    );
    expect(
      normalizeMarkdownHref(
        "adr/021-herdr-parity-web-workspaces.md",
        "docs/README.md",
      ),
    ).toBe("/docs/adr/021-herdr-parity-web-workspaces");
    expect(normalizeMarkdownHref("../LICENSE", "docs/license.md")).toBe(
      "/docs/license",
    );
    expect(normalizeMarkdownHref("#authentication", "docs/rest-api.md")).toBe(
      "#authentication",
    );
  });

  test("leaves external protocols untouched", () => {
    expect(normalizeMarkdownHref("https://example.com/x", "docs/faq.md")).toBe(
      "https://example.com/x",
    );
    expect(normalizeMarkdownHref("mailto:support@potenfyr.in", "docs/faq.md")).toBe(
      "mailto:support@potenfyr.in",
    );
  });
});
