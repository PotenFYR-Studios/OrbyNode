/**
 * Realtime file explorer: collapsible tree, live refresh on fs.change events,
 * drag a file onto a pane to drop its path into the terminal input.
 */

import { useCallback, useEffect, useState } from "react";
import type { RealtimeClient } from "./realtime";

export interface FileEntry {
  name: string;
  kind: "file" | "dir";
  size?: number;
}

const dragMime = "application/x-orbynode-path";

async function listFiles(projectId: number, path: string): Promise<FileEntry[]> {
  const res = await fetch(
    `/projects/${projectId}/files?path=${encodeURIComponent(path)}`,
  );
  if (!res.ok) return [];
  const raw = (await res.json()) as Array<{ name: string; kind?: string }>;
  return raw.map((e) => ({
    name: e.name,
    kind: e.kind === "dir" ? "dir" : "file",
  }));
}

interface Props {
  projectId: number;
  realtime: RealtimeClient;
  root?: string;
  depth?: number;
  onFileClick?: (path: string, entry: FileEntry) => void;
}

export function FileExplorer({ projectId, realtime, root = ".", depth = 0, onFileClick }: Props) {
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState(depth === 0);

  const refresh = useCallback(() => {
    void listFiles(projectId, root).then(setEntries);
  }, [projectId, root]);

  useEffect(refresh, [refresh]);

  // Live updates: any fs change on the project stream re-reads this dir.
  useEffect(() => {
    const remove = realtime.on("project", (ev) => {
      if (ev.etype === "fs.change") refresh();
    });
    realtime.sub("project");
    return remove;
  }, [realtime, refresh]);

  const toggle = (dir: string) => {
    setOpen((cur) => {
      const next = new Set(cur);
      if (next.has(dir)) next.delete(dir);
      else next.add(dir);
      return next;
    });
  };

  const join = (parent: string, name: string) =>
    parent === "." ? name : `${parent}/${name}`;

  if (!expanded) {
    return (
      <button type="button" className="tree-dir" onClick={() => setExpanded(true)}>
        <span className="caret" aria-hidden>▸</span> {root === "." ? "project" : root.split("/").pop()}
      </button>
    );
  }

  return (
    <div className="tree" role="tree">
      {depth === 0 && (
        <button type="button" className="tree-dir" onClick={() => setExpanded(false)}>
          <span className="caret" aria-hidden>▾</span> project
        </button>
      )}
      {entries.length === 0 && <div className="tree-empty">empty</div>}
      {entries.map((e) => {
        const path = join(root, e.name);
        if (e.kind === "dir") {
          return (
            <div key={path}>
              <button type="button" className="tree-dir" onClick={() => toggle(path)}>
                <span className="caret" aria-hidden>{open.has(path) ? "▾" : "▸"}</span> {e.name}
              </button>
              {open.has(path) && (
                <div className="tree-children">
                  <FileExplorer
                    projectId={projectId}
                    realtime={realtime}
                    root={path}
                    depth={depth + 1}
                    onFileClick={onFileClick}
                  />
                </div>
              )}
            </div>
          );
        }
        return (
          <div
            key={path}
            className="tree-file"
            role="treeitem"
            tabIndex={0}
            draggable
            onDragStart={(ev) => {
              ev.dataTransfer.setData(dragMime, path);
              ev.dataTransfer.effectAllowed = "copy";
            }}
            onClick={() => onFileClick?.(path, e)}
            onKeyDown={(ev) => {
              if (ev.key === "Enter") onFileClick?.(path, e);
            }}
          >
            <span className="file-icon" aria-hidden>▪</span> {e.name}
          </div>
        );
      })}
    </div>
  );
}
