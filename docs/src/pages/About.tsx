// About: mission, guarantees, studio and licensing - all from the README.
import { Link } from "react-router-dom";
import { ShieldCheck, Github, MessageSquare, Globe } from "lucide-react";

const GUARANTEES = [
  "One daemon owns real PTYs and durable state; clients are replaceable UIs, never the source of truth.",
  "Argon2id password hashing, session auth, throttling and per-project RBAC (owner, admin, operator, developer, viewer).",
  "CSRF protection and strict CSP on every API and UI surface; security is never weakened to make changes easier.",
  "Path containment guards every file and Git operation inside authorized project roots.",
  "Bounded queues, bounded replay and backpressure-safe realtime streams throughout - runaway work cannot exhaust the host.",
  "Durable state lives in SQLite with migrations; high-frequency transient state stays in memory by design.",
];

export default function About() {
  return (
    <div className="dot-backdrop">
      <div className="about-wrap">
        <p className="eyebrow">
          About the project
        </p>
        <h1>
          Agents deserve a <span className="grad-text">control plane</span>
        </h1>
        <p style={{ marginTop: "1rem" }}>
          OrbyNode is a self-hosted control plane for coding agents. It gives
          operators persistent terminals, durable workspaces, Git, tasks and
          remote machines behind one authenticated daemon - so agents keep
          working while browsers close and networks switch.
        </p>

        <h2>Core guarantees</h2>
        <ul className="guarantee-list">
          {GUARANTEES.map((g) => (
            <li key={g.slice(0, 24)}>
              <ShieldCheck size={16} />
              <span>{g}</span>
            </li>
          ))}
        </ul>

        <h2>PotenFYR Studios</h2>
        <p>
          OrbyNode is built by PotenFYR Studios, a small open-source studio.
          The org keeps its tools open and auditable under Apache-2.0 with the
          Commons Clause: free to use, self-host and embed, but nobody resells
          it as-is.
        </p>
        <div className="about-grid">
          <a
            href="https://github.com/PotenFYR-Studios"
            className="doc-card"
            target="_blank"
            rel="noopener noreferrer"
          >
            <span className="card-title" style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
              <Github size={16} />
              GitHub org
            </span>
            <span className="card-desc">Source, issues and releases.</span>
          </a>
          <a
            href="https://potenfyr.in/"
            className="doc-card"
            target="_blank"
            rel="noopener noreferrer"
          >
            <span className="card-title" style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
              <Globe size={16} />
              potenfyr.in
            </span>
            <span className="card-desc">The studio's home on the web.</span>
          </a>
          <a
            href="https://discord.com/invite/zUaN2FPBec"
            className="doc-card"
            target="_blank"
            rel="noopener noreferrer"
          >
            <span className="card-title" style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
              <MessageSquare size={16} />
              Support Discord
            </span>
            <span className="card-desc">Questions, help and updates.</span>
          </a>
        </div>

        <h2>License</h2>
        <p>
          OrbyNode is licensed under the{" "}
          <strong>Apache License 2.0 with the Commons Clause</strong>: free to
          use, study, modify, self-host and redistribute, and embedding it
          inside a larger product is welcome. Selling OrbyNode itself as a
          paid product or managed host is the one bright line. See{" "}
          <Link to="/docs/license">the license page</Link> and the{" "}
          <a
            href="https://github.com/PotenFYR-Studios/OrbyNode/blob/main/LICENSE"
            target="_blank"
            rel="noopener noreferrer"
          >
            LICENSE
          </a>{" "}
          file, which is authoritative.
        </p>

        <p style={{ marginTop: "3rem", textAlign: "center", fontFamily: "var(--mono)", fontSize: "0.78rem", color: "var(--faint)" }}>
          Made with &lt;3 by PotenFYR Studios
        </p>
      </div>
    </div>
  );
}
