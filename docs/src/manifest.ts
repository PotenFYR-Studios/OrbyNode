export type DocGroup =
  | "Getting started"
  | "Core concepts"
  | "Operations"
  | "Integrations"
  | "Contributing"
  | "Reference"
  | "Architecture decisions";

export type DocKind = "guide" | "adr";

export interface DocMeta {
  slug: string;
  title: string;
  description: string;
  group: DocGroup;
  kind: DocKind;
  source: string;
  route: string;
}

export interface PageMeta {
  title: string;
  description: string;
}

export const SITE_URL = "https://orbynode.docs.potenfyr.in";

export const DOC_GROUPS: readonly DocGroup[] = [
  "Getting started",
  "Core concepts",
  "Operations",
  "Integrations",
  "Contributing",
  "Reference",
  "Architecture decisions",
];

function guide(
  slug: string,
  title: string,
  group: Exclude<DocGroup, "Architecture decisions">,
  description: string,
  source = `docs/${slug}.md`,
): DocMeta {
  return {
    slug,
    title,
    description,
    group,
    kind: "guide",
    source,
    route: `/docs/${slug}`,
  };
}

function adr(slug: string, title: string, description: string): DocMeta {
  return {
    slug,
    title,
    description,
    group: "Architecture decisions",
    kind: "adr",
    source: `docs/adr/${slug}.md`,
    route: `/docs/adr/${slug}`,
  };
}

export const GUIDE_DOCS: readonly DocMeta[] = [
  guide(
    "getting-started",
    "Getting Started",
    "Getting started",
    "Install OrbyNode, create the first Owner, and launch a persistent coding-agent workspace.",
  ),
  guide(
    "installation",
    "Installation",
    "Getting started",
    "Install signed OrbyNode releases on Linux, macOS, and Windows.",
  ),
  guide(
    "releases",
    "Releases",
    "Getting started",
    "Verify release artifacts, upgrade safely, and roll back without losing durable state.",
  ),
  guide(
    "architecture",
    "Architecture",
    "Core concepts",
    "Understand OrbyNode's daemon-centric runtime, boundaries, crates, and data flow.",
    "ARCHITECTURE.md",
  ),
  guide(
    "realtime",
    "Realtime Protocol",
    "Core concepts",
    "Use sequenced snapshots, bounded replay, and backpressure-safe realtime streams.",
  ),
  guide(
    "agents",
    "Agents",
    "Core concepts",
    "Understand agent detection, native integrations, and Attention Center state.",
  ),
  guide(
    "security-model",
    "Security Model",
    "Core concepts",
    "Review trust boundaries, authentication, RBAC, CSRF protection, and audit guarantees.",
  ),
  guide(
    "remote-nodes",
    "Remote Nodes",
    "Core concepts",
    "Pair, operate, and revoke remote machines with durable node identity.",
  ),
  guide(
    "notifications",
    "Notifications",
    "Core concepts",
    "Route attention, workflow, and user-defined events without blocking agents.",
  ),
  guide(
    "performance",
    "Performance",
    "Core concepts",
    "Review resource bounds, performance baselines, and production measurement gates.",
  ),
  guide(
    "operations",
    "Operations",
    "Operations",
    "Run, monitor, back up, restore, and harden an OrbyNode daemon.",
  ),
  guide(
    "troubleshooting",
    "Troubleshooting",
    "Operations",
    "Diagnose installation, authentication, terminal, database, and remote-node failures.",
  ),
  guide(
    "rest-api",
    "REST API",
    "Integrations",
    "Integrate with OrbyNode's authenticated REST, WebSocket, MCP, and plugin surfaces.",
  ),
  guide(
    "workflows",
    "Workflows",
    "Integrations",
    "Define durable, approval-aware orchestration with bounded command execution.",
  ),
  guide(
    "frontend",
    "Frontend Guide",
    "Integrations",
    "Build OrbyNode interfaces with Vite, React, TypeScript, Bun, and owned Magic UI patterns.",
  ),
  guide(
    "contributing",
    "Contributing Guide",
    "Contributing",
    "Follow OrbyNode's contribution workflow, review rules, and commit discipline.",
  ),
  guide(
    "development",
    "Development Guide",
    "Contributing",
    "Build, run, test, debug, and package OrbyNode from source.",
  ),
  guide(
    "testing",
    "Testing Guide",
    "Contributing",
    "Use deterministic unit, integration, filesystem, security, and realtime tests.",
  ),
  guide(
    "changelog",
    "Changelog",
    "Contributing",
    "Track user-visible OrbyNode changes and release history.",
  ),
  guide(
    "roadmap",
    "Roadmap",
    "Contributing",
    "Review completed milestones, current release focus, and explicit non-goals.",
    "ROADMAP.md",
  ),
  guide(
    "v1-readiness",
    "v1.0 Readiness",
    "Contributing",
    "Track clean-machine guarantees required before the v1.0 label.",
  ),
  guide(
    "faq",
    "Frequently Asked Questions",
    "Reference",
    "Find concise answers about OrbyNode security, architecture, operations, and licensing.",
  ),
  guide(
    "license",
    "License",
    "Reference",
    "Understand OrbyNode's Apache-2.0 license with Commons Clause.",
  ),
  guide(
    "security",
    "Security Policy",
    "Reference",
    "Report vulnerabilities privately and review supported security expectations.",
    "SECURITY.md",
  ),
  guide(
    "contributing-policy",
    "Repository Contribution Policy",
    "Reference",
    "Review the repository-level contribution process and required checks.",
    "CONTRIBUTING.md",
  ),
];

export const ADR_DOCS: readonly DocMeta[] = [
  adr("001-daemon-architecture", "ADR 001 — Daemon Architecture", "Why the daemon owns process lifetime and durable runtime state."),
  adr("002-pty-abstraction", "ADR 002 — PTY Abstraction", "How OrbyNode provides real cross-platform pseudo-terminals."),
  adr("003-api-transport", "ADR 003 — API Transport", "Why browser and automation clients use authenticated HTTP and WebSocket transport."),
  adr("004-database-storage", "ADR 004 — Database / Storage", "Why SQLite is the durable source of truth."),
  adr("005-auth-session-model", "ADR 005 — Authentication / Session Model", "How sessions, credentials, and authorization boundaries work."),
  adr("006-embedded-frontend", "ADR 006 — Embedded Frontend Delivery", "Why the daemon embeds static frontend assets."),
  adr("007-desktop-daemon-separation", "ADR 007 — Desktop / Daemon Separation", "Why the desktop shell remains a replaceable daemon client."),
  adr("008-attention-center", "ADR 008 — Attention Center", "How agent states roll up into actionable attention."),
  adr("009-realtime-protocol", "ADR 009 — Realtime Protocol", "How snapshots, sequence numbers, replay, and overflow recovery fit together."),
  adr("010-remote-node-identity", "ADR 010 — Remote Node Identity and Pairing", "How remote machines establish durable, revocable identity."),
  adr("011-api-interface-stability", "ADR 011 — API Interface Stability", "The canonical policy for stable external interfaces."),
  adr("011-platform-interfaces", "ADR 011 — Platform Interfaces", "Historical platform-interface decision superseded by the canonical ADR 011."),
  adr("012-performance-baseline", "ADR 012 — Performance Baseline", "The minimum performance evidence required for release."),
  adr("013-security-hardening", "ADR 013 — Security Hardening Baseline", "Baseline controls for deployment and release security."),
  adr("014-release-engineering", "ADR 014 — Release Engineering", "How OrbyNode builds, signs, and publishes releases."),
  adr("015-ui-design-motion", "ADR 015 — UI Design and Motion", "The interface stack, visual system, and reduced-motion rules."),
  adr("016-multi-user-collaboration", "ADR 016 — Multi-User Collaboration", "How users share projects under server-enforced authorization."),
  adr("017-resource-efficiency", "ADR 017 — Resource Efficiency", "Why queues, journals, and sessions stay bounded."),
  adr("018-plugin-isolation", "ADR 018 — Plugin Isolation", "Why plugins consume APIs instead of loading native code into the daemon."),
  adr("019-secrets-handling", "ADR 019 — Secrets Handling", "How OrbyNode stores, reveals, and redacts secrets."),
  adr("020-update-signing-model", "ADR 020 — Update and Signing Model", "How keyless signatures and installer verification protect updates."),
  adr("021-herdr-parity-web-workspaces", "ADR 021 — Web Workspaces and Recovery", "How workspaces, panes, scrollback, and agent sessions recover after crashes."),
];

export const DOCS: readonly DocMeta[] = [...GUIDE_DOCS, ...ADR_DOCS];

export function validateManifest(docs: readonly DocMeta[]): void {
  for (const field of ["source", "route"] as const) {
    const seen = new Set<string>();
    for (const doc of docs) {
      if (seen.has(doc[field])) {
        throw new Error(`duplicate documentation ${field}: ${doc[field]}`);
      }
      seen.add(doc[field]);
    }
  }

  const scopedSlugs = new Set<string>();
  for (const doc of docs) {
    const key = `${doc.kind}:${doc.slug}`;
    if (scopedSlugs.has(key)) {
      throw new Error(`duplicate documentation slug: ${key}`);
    }
    scopedSlugs.add(key);
  }
}

validateManifest(DOCS);

function cleanPath(pathname: string): string {
  const path = pathname.split(/[?#]/, 1)[0].replace(/\/+$/, "");
  return path || "/";
}

export function docBySlug(slug: string, kind: DocKind = "guide"): DocMeta | undefined {
  return DOCS.find((doc) => doc.kind === kind && doc.slug === slug);
}

export function docByRoute(pathname: string): DocMeta | undefined {
  const path = cleanPath(pathname);
  return DOCS.find((doc) => doc.route === path);
}

export const SITE_ROUTES: readonly string[] = [
  "/",
  "/docs",
  "/about",
  ...DOCS.map((doc) => doc.route),
];

export function routeMeta(pathname: string): PageMeta {
  const path = cleanPath(pathname);
  if (path === "/") {
    return {
      title: "OrbyNode — Persistent Control Plane for Coding Agents",
      description:
        "Run persistent coding agents, terminals, files, Git tasks, and remote machines from one self-hosted control plane.",
    };
  }
  if (path === "/docs") {
    return {
      title: "Documentation | OrbyNode",
      description:
        "Install, operate, secure, extend, and contribute to OrbyNode.",
    };
  }
  if (path === "/about") {
    return {
      title: "About | OrbyNode",
      description:
        "Why OrbyNode exists, what it guarantees, and who builds it.",
    };
  }

  const doc = docByRoute(path);
  if (doc) {
    return {
      title: `${doc.title} | OrbyNode Documentation`,
      description: doc.description,
    };
  }

  return {
    title: "Page not found | OrbyNode",
    description: "This OrbyNode documentation page could not be found.",
  };
}

const ROOT_LINKS: Readonly<Record<string, string>> = {
  "ARCHITECTURE.md": "/docs/architecture",
  "ROADMAP.md": "/docs/roadmap",
  "SECURITY.md": "/docs/security",
  "CONTRIBUTING.md": "/docs/contributing-policy",
  LICENSE: "/docs/license",
};

function normalizePath(path: string): string {
  const result: string[] = [];
  for (const segment of path.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") result.pop();
    else result.push(segment);
  }
  return result.join("/");
}

export function normalizeMarkdownHref(href: string, source: string): string {
  if (!href || href.startsWith("#") || /^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(href)) {
    return href;
  }
  if (href.startsWith("/")) return href;

  const hashIndex = href.indexOf("#");
  const hash = hashIndex >= 0 ? href.slice(hashIndex) : "";
  const pathWithQuery = hashIndex >= 0 ? href.slice(0, hashIndex) : href;
  const queryIndex = pathWithQuery.indexOf("?");
  const query = queryIndex >= 0 ? pathWithQuery.slice(queryIndex) : "";
  const relativePath = queryIndex >= 0 ? pathWithQuery.slice(0, queryIndex) : pathWithQuery;
  const sourceDir = source.includes("/") ? source.slice(0, source.lastIndexOf("/")) : "";
  const resolved = normalizePath(`${sourceDir}/${relativePath}`);

  const direct = DOCS.find((doc) => doc.source === resolved)?.route;
  if (direct) return `${direct}${query}${hash}`;

  const root = ROOT_LINKS[resolved];
  if (root) return `${root}${query}${hash}`;

  if (resolved === "docs/adr" || resolved === "docs/adr/") {
    return `/docs#architecture-decisions${hash}`;
  }

  return href;
}
