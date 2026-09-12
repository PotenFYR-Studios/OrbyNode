import { useEffect, useState } from "react";
import { api, type Health, type Version } from "./api";

interface State {
  health?: Health;
  version?: Version;
  error?: string;
}

export function App() {
  const [state, setState] = useState<State>({});

  useEffect(() => {
    Promise.all([api.health(), api.version()])
      .then(([health, version]) => setState({ health, version }))
      .catch((e: unknown) => setState({ error: String(e) }));
  }, []);

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", maxWidth: 640, margin: "4rem auto", padding: "0 1rem" }}>
      <h1>OrbyNode</h1>
      <p>The self-hosted control plane for coding agents.</p>
      {state.error && <p role="alert">Daemon unreachable: {state.error}</p>}
      {state.health && (
        <dl>
          <dt>Status</dt>
          <dd>{state.health.status}</dd>
          <dt>Uptime</dt>
          <dd>{state.health.uptime_secs}s</dd>
          <dt>Version</dt>
          <dd>
            {state.version?.name} {state.version?.version}
          </dd>
        </dl>
      )}
    </main>
  );
}
