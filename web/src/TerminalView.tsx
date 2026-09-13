/**
 * Terminal view: renders a pane's PTY output as text with ANSI-color class
 * mapping, auto-scroll, and input forwarding. Pure DOM typing keeps the
 * bundle small; xterm.js can replace this component later without touching
 * the data flow (same /panes/:id/output + terminal:<id> stream).
 */

import { useEffect, useRef, useState } from "react";
import { workspaceApi } from "./workspace";
import type { RealtimeClient } from "./realtime";

/** Strip ANSI escapes; keep text. SGR color codes map to span classes. */
function renderAnsi(text: string): { text: string; cls: string }[] {
  const out: { text: string; cls: string }[] = [];
  const re = /\x1b\[(\d+;?\d*)m/g;
  let last = 0;
  let cls = "";
  for (let m = re.exec(text); m !== null; m = re.exec(text)) {
    if (m.index > last) out.push({ text: text.slice(last, m.index), cls });
    const raw = m[1] ?? "";
    const code = raw.split(";")[0] ?? "";
    if (code === "0" || code === "00") cls = "";
    else if (code.length === 2 && code.startsWith("3")) {
      const n = Number(code.slice(1));
      cls = n >= 1 && n <= 6 ? `ansi-${n}` : "";
    }
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last), cls });
  return out;
}

interface Props {
  paneId: number;
  terminalId: number | null;
  realtime: RealtimeClient;
  focused: boolean;
}

export function TerminalView({ paneId, terminalId, realtime, focused }: Props) {
  const [lines, setLines] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [history, setHistory] = useState<string[]>([]);
  const [histIdx, setHistIdx] = useState(-1);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Journal replay once (bounded), then live via realtime stream.
  useEffect(() => {
    let alive = true;
    void workspaceApi.output(paneId, -1).then((res) => {
      if (!alive) return;
      const joined = res.chunks.map((c) => c.text).join("");
      setLines(joined.split(/(?<=\n)/).slice(-2000));
    });
    return () => {
      alive = false;
    };
  }, [paneId]);

  useEffect(() => {
    if (!terminalId) return;
    const stream = `terminal:${terminalId}`;
    const remove = realtime.onBinary((s, payload) => {
      if (s !== stream) return;
      const text = new TextDecoder().decode(payload);
      setLines((cur) => {
        const next = text.split(/(?<=\n)/);
        const merged = [...cur, ...next];
        return merged.length > 2400 ? merged.slice(merged.length - 2000) : merged;
      });
    });
    return remove;
  }, [terminalId, realtime]);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);

  const submit = (data: string) => {
    void workspaceApi.sendInput(paneId, data);
    setHistory((h) => (data.trim() ? [data.trim(), ...h].slice(0, 100) : h));
    setHistIdx(-1);
  };

  const onKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      submit(`${input}\n`);
      setInput("");
    } else if (e.key === "ArrowUp" && history.length > 0) {
      e.preventDefault();
      const next = Math.min(histIdx + 1, history.length - 1);
      setHistIdx(next);
      setInput(history[next] ?? "");
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const next = Math.max(histIdx - 1, -1);
      setHistIdx(next);
      setInput(next === -1 ? "" : (history[next] ?? ""));
    } else if (e.key === "c" && e.ctrlKey) {
      submit("\u0003");
      setInput("");
    } else if (e.key === "d" && e.ctrlKey) {
      submit("\u0004");
      setInput("");
    }
  };

  // Drag-drop: file path from the explorer becomes part of the input line.
  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    const path = e.dataTransfer.getData("application/x-orbynode-path");
    if (path) setInput((cur) => cur + path);
    inputRef.current?.focus();
  };

  return (
    <div
      className={`terminal-view ${focused ? "focused" : ""}`}
      onClick={() => inputRef.current?.focus()}
      onDrop={onDrop}
      onDragOver={(e) => e.preventDefault()}
    >
      <div className="terminal-scroll" ref={scrollRef} aria-live="polite">
        {lines.map((line, i) => (
          <div key={i} className="terminal-line">
            {renderAnsi(line).map((seg, j) => (
              <span key={j} className={seg.cls ? `c${seg.cls}` : undefined}>
                {seg.text}
              </span>
            ))}
          </div>
        ))}
      </div>
      <div className="terminal-input-row">
        <span className="prompt-dot" aria-hidden />
        <input
          ref={inputRef}
          className="terminal-input"
          value={input}
          placeholder={focused ? "type a command…" : "click pane to focus"}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKey}
          aria-label={`Pane ${paneId} input`}
        />
      </div>
    </div>
  );
}
