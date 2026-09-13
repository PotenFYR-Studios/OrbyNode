// Landing page: compact control-room hero with verified install command,
// capability grid, architecture flow, project stats, focused CTA. Every
// claim mirrors the repository README/docs - nothing invented. Effects
// confined to the hero.
import { Link } from "react-router-dom";
import {
  TerminalSquare,
  Activity,
  Bot,
  FolderGit2,
  ListChecks,
  Network,
  ShieldCheck,
} from "lucide-react";
import { GlowOrb, NumberTicker, SpotlightCard } from "../components/MagicUI";
import { CopyButton } from "../components/CopyButton";

const INSTALL_CMD = "curl -fsSL https://orbynode.docs.potenfyr.in/install.sh | sh";
const INSTALL_PS = "irm https://orbynode.docs.potenfyr.in/install.ps1 | iex";

const CAPABILITIES = [
  {
    icon: <TerminalSquare size={17} />,
    title: "Persistent PTYs",
    body: "Real terminals owned by the daemon survive disconnects, UI restarts and network switches.",
  },
  {
    icon: <Activity size={17} />,
    title: "Realtime state",
    body: "Sequenced snapshots, bounded replay and backpressure-safe event streams with no polling.",
  },
  {
    icon: <Bot size={17} />,
    title: "Agent awareness",
    body: "Working, waiting, approval and failure states detected across every paired machine.",
  },
  {
    icon: <FolderGit2 size={17} />,
    title: "Files and Git",
    body: "Inspect diffs inside authorized project roots; tasks get isolated worktrees with a review path.",
  },
  {
    icon: <ListChecks size={17} />,
    title: "Durable tasks",
    body: "Approval-aware workflows with bounded command execution and durable state in SQLite.",
  },
  {
    icon: <Network size={17} />,
    title: "Remote nodes",
    body: "Pair machines with durable node identity; aggregate agents without losing host boundaries.",
  },
];

const FLOW = [
  {
    step: "01",
    title: "Daemon",
    body: "One self-hosted daemon owns PTYs, state and authorization. Clients are replaceable.",
  },
  {
    step: "02",
    title: "Realtime bus",
    body: "Every mutation flows through the sequenced event bus; clients snapshot then subscribe.",
  },
  {
    step: "03",
    title: "Agents",
    body: "Terminal heuristics and native integrations turn raw output into agent attention states.",
  },
];

const STATS = [
  { value: 22, suffix: "", label: "ADRs" },
  { value: 15, suffix: "", label: "crates" },
  { value: 21, suffix: "", label: "milestones shipped" },
  { value: 1, suffix: " daemon", label: "self-hosted" },
];

export default function Home() {
  return (
    <>
      <section className="hero dot-backdrop">
        <GlowOrb className="-left-32 -top-24" color="rgba(34, 211, 238, 0.14)" />
        <GlowOrb className="right-[-8rem] top-10" color="rgba(139, 92, 246, 0.14)" />
        <div className="hero-inner">
          <p className="eyebrow">Self-hosted · local-first · persistent agents</p>
          <h1>
            The control plane for <span className="grad-text">coding agents</span>.
          </h1>
          <p className="hero-lede">
            OrbyNode owns real PTYs, durable workspaces, Git, tasks and remote
            nodes in one daemon. Close the browser, switch networks, reopen a
            desktop shell later - your agents keep working.
          </p>
          <div className="hero-cta">
            <Link to="/docs/getting-started" className="btn btn-primary">
              Get started
            </Link>
            <Link to="/docs" className="btn btn-ghost">
              Read the docs
            </Link>
          </div>
          <div className="install-panel border-beam">
            <code className="install-cmd">
              <span className="dollar">$ </span>
              <span className="accent">{INSTALL_CMD}</span>
            </code>
            <CopyButton text={INSTALL_CMD} />
          </div>
          <p className="install-alt" style={{ marginTop: "0.7rem", fontSize: "0.78rem", color: "var(--faint)", fontFamily: "var(--mono)" }}>
            Windows: <code>{INSTALL_PS}</code>
          </p>
        </div>
      </section>

      <section className="home-section">
        <h2>One control plane, not another terminal wrapper</h2>
        <p className="section-sub">
          Everything an operator of coding agents needs, in one auditable daemon.
        </p>
        <div className="cap-grid">
          {CAPABILITIES.map((cap) => (
            <SpotlightCard key={cap.title} className="cap-card">
              <h3>
                {cap.icon}
                {cap.title}
              </h3>
              <p>{cap.body}</p>
            </SpotlightCard>
          ))}
        </div>
      </section>

      <section className="home-section">
        <h2>How it fits together</h2>
        <p className="section-sub">
          Daemon-centric runtime: clients come and go, durable state and the
          event bus stay.
        </p>
        <div className="flow-grid">
          {FLOW.map((f) => (
            <div key={f.step} className="flow-card doc-card">
              <span className="flow-step">{f.step}</span>
              <h3>{f.title}</h3>
              <p>{f.body}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="home-section">
        <h2>Project at a glance</h2>
        <p className="section-sub">Milestone-driven, ADR-governed development.</p>
        <div className="stat-grid">
          {STATS.map((stat) => (
            <div key={stat.label} className="stat-card doc-card">
              <div className="stat-value">
                <NumberTicker value={stat.value} />
                {stat.suffix}
              </div>
              <div className="stat-label">{stat.label}</div>
            </div>
          ))}
        </div>
      </section>

      <section className="home-section home-cta">
        <h2 className="grad-text">Safe by default</h2>
        <p className="section-sub" style={{ marginInline: "auto" }}>
          Argon2id sessions, RBAC, CSRF protection, path containment and
          bounded buffers throughout. Security decisions are never weakened to
          make changes easier.
        </p>
        <div style={{ display: "flex", justifyContent: "center", gap: "0.8rem", flexWrap: "wrap" }}>
          <Link to="/docs/installation" className="btn btn-primary">
            Install OrbyNode
          </Link>
          <Link to="/docs/security-model" className="btn btn-ghost">
            <ShieldCheck size={15} />
            Read the security model
          </Link>
        </div>
      </section>
    </>
  );
}
