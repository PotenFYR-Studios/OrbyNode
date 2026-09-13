// Content layer: binds the pure manifest (manifest.ts) to the actual
// Markdown bodies. Vite's import.meta.glob inlines every doc at build
// time, so the deployed site never fetches Markdown separately and the
// files stay exactly where the GitHub README links to them.

import { DOCS, type DocMeta } from "./manifest";

export {
  ADR_DOCS,
  DOC_GROUPS,
  DOCS,
  GUIDE_DOCS,
  SITE_ROUTES,
  SITE_URL,
  docByRoute,
  docBySlug,
  normalizeMarkdownHref,
  routeMeta,
  validateManifest,
} from "./manifest";
export type { DocGroup, DocKind, DocMeta, PageMeta } from "./manifest";

const guideBodies = import.meta.glob("../**/*.md", {
  eager: true,
  query: "?raw",
  import: "default",
}) as Record<string, string>;

const rootBodies = import.meta.glob(
  "../../{ARCHITECTURE,ROADMAP,SECURITY,CONTRIBUTING}.md",
  {
    eager: true,
    query: "?raw",
    import: "default",
  },
) as Record<string, string>;

const bodiesBySource = new Map<string, string>();
for (const [key, body] of Object.entries(guideBodies)) {
  // Glob keys are relative to this file: ../installation.md or ../adr/001-....md.
  bodiesBySource.set(`docs/${key.replace(/^\.\.\//, "")}`, body);
}
for (const [key, body] of Object.entries(rootBodies)) {
  bodiesBySource.set(key.replace(/^(\.\.\/)+/, ""), body);
}

// Build-time completeness check: every manifest entry must have a body.
// A missing file means the manifest and the repo drifted apart.
for (const doc of DOCS) {
  if (!bodiesBySource.has(doc.source)) {
    throw new Error(`docs manifest references missing file: ${doc.source}`);
  }
}

export function docBody(doc: DocMeta): string {
  const body = bodiesBySource.get(doc.source);
  if (body === undefined) {
    throw new Error(`no content loaded for ${doc.source}`);
  }
  return body;
}

export function bodyByRoute(pathname: string): string | undefined {
  for (const doc of DOCS) {
    if (doc.route === pathname.split(/[?#]/, 1)[0].replace(/\/+$/, "")) {
      return docBody(doc);
    }
  }
  return undefined;
}
