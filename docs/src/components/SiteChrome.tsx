// Site chrome: sticky blur header and 3-zone footer. Mobile drawer below
// 760px. Brand mark is the inline favicon so it works on any base path.
import { useState } from "react";
import { Link, NavLink } from "react-router-dom";
import { Menu, X } from "lucide-react";

const NAV = [
  { to: "/", label: "Home", end: true },
  { to: "/docs", label: "Docs", end: false },
  { to: "/about", label: "About", end: false },
];

const EXTERNAL = [
  { href: "https://potenfyr.in/", label: "Website" },
  { href: "https://discord.com/invite/zUaN2FPBec", label: "Discord" },
  { href: "https://github.com/PotenFYR-Studios/OrbyNode", label: "GitHub" },
];

function Brand() {
  return (
    <Link to="/" className="brand">
      <img src="/favicon.svg" alt="" width="22" height="22" className="brand-mark" />
      Orby<span>Node</span>
    </Link>
  );
}

export function SiteHeader() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <header className="site-header">
        <div className="header-inner">
          <Brand />
          <nav className="nav-links" aria-label="Primary">
            {NAV.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}
              >
                {item.label}
              </NavLink>
            ))}
            <a className="nav-cta" href="https://github.com/PotenFYR-Studios/OrbyNode">
              GitHub
            </a>
            <button
              type="button"
              className="menu-btn"
              aria-expanded={open}
              aria-controls="mobile-nav"
              aria-label={open ? "Close menu" : "Open menu"}
              onClick={() => setOpen((v) => !v)}
            >
              {open ? <X size={18} /> : <Menu size={18} />}
            </button>
          </nav>
        </div>
      </header>
      <div id="mobile-nav" className={`mobile-nav${open ? " open" : ""}`}>
        <nav aria-label="Mobile">
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) => `nav-link${isActive ? " active" : ""}`}
              onClick={() => setOpen(false)}
            >
              {item.label}
            </NavLink>
          ))}
          {EXTERNAL.map((item) => (
            <a key={item.href} className="nav-link" href={item.href}>
              {item.label}
            </a>
          ))}
        </nav>
      </div>
    </>
  );
}

export function SiteFooter() {
  return (
    <footer className="site-footer">
      <div className="footer-inner">
        <div className="footer-brand">
          <Brand />
          <p className="footer-blurb">
            Self-hosted control plane for coding agents: persistent PTYs,
            durable workspaces, Git, tasks and remote nodes in one daemon.
          </p>
        </div>
        <div className="footer-col">
          <h3>Documentation</h3>
          <ul>
            <li><Link to="/docs">Docs index</Link></li>
            <li><Link to="/docs/getting-started">Getting started</Link></li>
            <li><Link to="/docs/installation">Installation</Link></li>
            <li><Link to="/docs/rest-api">REST API</Link></li>
            <li><Link to="/docs/adr/001-daemon-architecture">ADRs</Link></li>
          </ul>
        </div>
        <div className="footer-col">
          <h3>Project</h3>
          <ul>
            {EXTERNAL.map((item) => (
              <li key={item.href}>
                <a href={item.href} target="_blank" rel="noopener noreferrer">
                  {item.label}
                </a>
              </li>
            ))}
            <li><Link to="/about">About</Link></li>
            <li><Link to="/docs/license">License</Link></li>
          </ul>
        </div>
      </div>
      <div className="footer-legal">
        <span>© 2026 PotenFYR Studios · Apache-2.0 with Commons Clause</span>
        <span>Made with &lt;3 by PotenFYR Studios</span>
      </div>
    </footer>
  );
}
