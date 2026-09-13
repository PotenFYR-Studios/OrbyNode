/**
 * Plugins page: discover, enable/disable, inspect plugin manifests (ADR 018).
 * Manifests only - no untrusted execution; marketplace deferred (Plan §145).
 */

import { useEffect, useState } from "react";

export interface PluginRecord {
  id: string;
  name: string;
  manifest: string;
  enabled: boolean;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) throw new Error(`${path}: ${res.status}`);
  return res.json() as Promise<T>;
}

export function PluginsPage() {
  const [plugins, setPlugins] = useState<PluginRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);

  const reload = () => {
    getJson<PluginRecord[]>("/plugins")
      .then(setPlugins)
      .catch((e: unknown) => setError(String(e)));
  };

  useEffect(reload, []);

  const setEnabled = (id: string, enabled: boolean) => {
    void fetch(`/plugins/${encodeURIComponent(id)}`, {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ enabled }),
    })
      .then((res) => {
        if (!res.ok) throw new Error(`patch: ${res.status}`);
        reload();
      })
      .catch((e: unknown) => setError(String(e)));
  };

  const selectedPlugin = plugins.find((p) => p.id === selected) ?? null;

  return (
    <div className="plugins-page">
      <h2>Plugins</h2>
      <p className="page-sub">
        Manifest-level plugins are records with declared capabilities; execution
        stays disabled until the plugin runtime ships (ADR 018).
      </p>
      {error && <p role="alert" className="error-banner">{error}</p>}
      <div className="plugin-layout">
        <ul className="plugin-list">
          {plugins.map((p) => (
            <li key={p.id}>
              <button
                type="button"
                className={`plugin-item ${p.id === selected ? "active" : ""}`}
                onClick={() => setSelected(p.id)}
              >
                <span className={`plugin-state ${p.enabled ? "on" : "off"}`} aria-hidden />
                {p.name}
                <span className="badge">{p.enabled ? "enabled" : "disabled"}</span>
              </button>
            </li>
          ))}
          {plugins.length === 0 && <li className="ws-empty">No plugins registered.</li>}
        </ul>
        {selectedPlugin && (
          <section className="plugin-detail">
            <h3>{selectedPlugin.name}</h3>
            <div className="plugin-actions">
              <button
                type="button"
                className="btn btn-primary"
                disabled={selectedPlugin.enabled}
                onClick={() => setEnabled(selectedPlugin.id, true)}
              >
                Enable
              </button>
              <button
                type="button"
                className="btn"
                disabled={!selectedPlugin.enabled}
                onClick={() => setEnabled(selectedPlugin.id, false)}
              >
                Disable
              </button>
            </div>
            <pre className="plugin-manifest">{selectedPlugin.manifest}</pre>
          </section>
        )}
      </div>
    </div>
  );
}
