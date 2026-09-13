// Docs hub: grouped card grid of every docs page.
import { Link } from "react-router-dom";
import { ArrowUpRight } from "lucide-react";
import { DOCS, DOC_GROUPS } from "../content";

export default function DocsHub() {
  return (
    <div className="dot-backdrop">
      <div className="hub-wrap">
        <p className="eyebrow">OrbyNode docs</p>
        <h1>
          OrbyNode <span className="grad-text">Documentation</span>
        </h1>
        <p className="hub-lede">
          Everything about installing, operating and integrating the
          self-hosted control plane for coding agents - from the first
          curl one-liner to remote nodes and workflows.
        </p>

        {DOC_GROUPS.map((group) => (
          <section key={group} className="hub-group" aria-labelledby={`group-${group}`}>
            <h2 id={`group-${group}`} className="hub-group-title">
              {group}
            </h2>
            <div className="hub-grid">
              {DOCS.filter((doc) => doc.group === group).map((doc) => (
                <Link key={doc.route} to={doc.route} className="doc-card">
                  <span className="card-title" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    {doc.title}
                    <ArrowUpRight size={14} style={{ color: "var(--faint)" }} />
                  </span>
                  <span className="card-desc">{doc.description}</span>
                </Link>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
