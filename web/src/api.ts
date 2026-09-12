/** Typed client for the daemon REST API. */

export interface Health {
  status: string;
  uptime_secs: number;
}

export interface Version {
  name: string;
  version: string;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) throw new Error(`${path}: ${res.status}`);
  return res.json() as Promise<T>;
}

export const api = {
  health: () => getJson<Health>("/health"),
  version: () => getJson<Version>("/version"),
};
