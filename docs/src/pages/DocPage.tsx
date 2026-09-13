// Docs reader: grouped sidebar + article + right TOC with active cyan bar,
// prev/next pagination, anchor scrolling on hash navigation.
import { useEffect, useMemo, useState } from "react";
import { Link, NavLink, useLocation, useParams } from "react-router-dom";
import { DOCS, DOC_GROUPS, docBody, docBySlug } from "../content";
import { renderMarkdown, type Heading } from "../components/Markdown";
import NotFound from "./NotFound";

function SidebarNav({ active }: { active: string }) {
  return (
    <nav aria-label="Docs sections">
      {DOC_GROUPS.map((group) => (
        <details key={group} className="side-group" open>
          <summary className="side-title">{group}</summary>
          {DOCS.filter((doc) => doc.group === group).map((doc) => (
            <NavLink
              key={doc.route}
              to={doc.route}
              className={({ isActive }) => `side-link${isActive ? " active" : ""}`}
            >
              {doc.title}
            </NavLink>
          ))}
        </details>
      ))}
    </nav>
  );
}

function Toc({ headings }: { headings: Heading[] }) {
  const [activeId, setActiveId] = useState("");
  const tocItems = useMemo(
    () => headings.filter((h) => h.level === 2 || h.level === 3),
    [headings],
  );

  useEffect(() => {
    if (tocItems.length === 0) return;
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setActiveId(entry.target.id);
            break;
          }
        }
      },
      { rootMargin: "-80px 0px -65% 0px" },
    );
    for (const h of tocItems) {
      const el = document.getElementById(h.id);
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  }, [tocItems]);

  if (tocItems.length < 3) return null;
  return (
    <nav className="toc" aria-label="On this page">
      <div className="toc-title">On this page</div>
      {tocItems.map((h) => (
        <a
          key={h.id}
          href={`#${h.id}`}
          className={`toc-item toc-h${h.level}${activeId === h.id ? " active" : ""}`}
          aria-current={activeId === h.id ? "true" : undefined}
        >
          {h.text}
        </a>
      ))}
    </nav>
  );
}

export default function DocPage() {
  const params = useParams();
  const location = useLocation();
  const doc =
    params.kind === "adr" || location.pathname.startsWith("/docs/adr/")
      ? docBySlug(params.slug ?? "", "adr")
      : docBySlug(params.slug ?? "", "guide");

  const { nodes, headings } = useMemo(
    () => (doc ? renderMarkdown(docBody(doc), doc.source) : { nodes: null, headings: [] }),
    [doc],
  );

  // Anchor scrolling for palette/TOC links.
  useEffect(() => {
    if (!location.hash) return;
    const el = document.getElementById(location.hash.slice(1));
    if (el) el.scrollIntoView({ block: "start" });
  }, [location.hash, nodes]);

  if (!doc) return <NotFound />;

  const idx = DOCS.findIndex((d) => d.route === doc.route);
  const prev = idx > 0 ? DOCS[idx - 1] : undefined;
  const next = idx < DOCS.length - 1 ? DOCS[idx + 1] : undefined;

  return (
    <div className="doc-layout">
      <aside className="doc-sidebar">
        <SidebarNav active={doc.route} />
      </aside>

      <article className="article">
        <header className="article-header">
          <span className="kind-badge">{doc.kind === "adr" ? "ADR" : "GUIDE"}</span>
          <h1>{doc.title}</h1>
          <p className="lede">{doc.description}</p>
        </header>

        <div className="article-body">{nodes}</div>

        <nav className="doc-pager" aria-label="Pagination">
          {prev ? (
            <Link className="pager-link prev" to={prev.route}>
              <span className="dir">Previous</span>
              <span className="page">{prev.title}</span>
            </Link>
          ) : (
            <span />
          )}
          {next ? (
            <Link className="pager-link next" to={next.route}>
              <span className="dir">Next</span>
              <span className="page">{next.title}</span>
            </Link>
          ) : (
            <span />
          )}
        </nav>
      </article>

      <Toc headings={headings} />
    </div>
  );
}
