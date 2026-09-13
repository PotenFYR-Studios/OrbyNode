// Keeps served artifacts in lockstep with their repo sources:
//   scripts/install.sh  -> public/install.sh   (the curl one-liner target)
//   scripts/install.ps1 -> public/install.ps1  (the irm one-liner target)
// Runs before every build. Copy is guarded: both sources must be
// non-trivially sized and carry the OrbyNode marker, so a refactor that
// accidentally empties them can never ship a broken installer page.
import { copyFileSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const repoRoot = resolve(root, "..");

const MIN_BYTES = 200;
const MARKER = "OrbyNode";

function syncInstaller(sourceName: string, targetName: string): void {
  const src = resolve(repoRoot, "scripts", sourceName);
  const dst = resolve(root, "public", targetName);

  let size = 0;
  try {
    size = statSync(src).size;
  } catch {
    throw new Error(`scripts/${sourceName} is missing; refusing to publish the docs copy`);
  }
  if (size < MIN_BYTES) {
    throw new Error(`scripts/${sourceName} is only ${size} bytes; refusing to publish it`);
  }
  if (!readFileSync(src, "utf8").includes(MARKER)) {
    throw new Error(`scripts/${sourceName} lost its "${MARKER}" marker; refusing to publish it`);
  }

  mkdirSync(resolve(root, "public"), { recursive: true });
  copyFileSync(src, dst);
  console.log(`[sync-assets] scripts/${sourceName} -> docs/public/${targetName} (${size} bytes)`);
}

syncInstaller("install.sh", "install.sh");
syncInstaller("install.ps1", "install.ps1");
