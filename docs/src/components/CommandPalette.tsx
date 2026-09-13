// OrbyNode docs command palette: Ctrl/Cmd+K searchable index of all
// documentation pages and headings. Keyboard navigation, Escape/overlay close,
// useful empty state, focus handling. No polling, no external deps.
import {
  useEffect,
  useId,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { useNavigate } from "react-router-dom";
import { Search } from "lucide-react";
import { DOCS, docBody } from "../content";
import { slugify } from "./Markdown";
import { filterPaletteEntries, type PaletteEntry } from "./palette";

function extractHeadings(markdown: string): { text: string; level: number }[] {
  const headings: { text: string; level: number }[] = [];
  for (const line of markdown.split("\n")) {
    const match = line.match(/^(#{1,4})\s+(.+)$/);
    if (match) {
      headings.push({ level: match[1].length, text: match[2].trim() });
    }
  }
  return headings;
}

/** Build a flat, searchable list of every page and its headings. */
export function buildPaletteEntries(): PaletteEntry[] {
  const entries: PaletteEntry[] = [];

  for (const doc of DOCS) {
    entries.push({
      title: doc.title,
      description: doc.description,
      route: doc.route,
      group: doc.group,
      kind: "page",
    });

    const body = docBody(doc);
    const headings = extractHeadings(body);
    for (const heading of headings) {
      entries.push({
        title: heading.text,
        description: `${doc.title} — ${heading.text}`,
        route: `${doc.route}#${slugify(heading.text)}`,
        group: doc.title,
        kind: "heading",
      });
    }
  }

  return entries;
}

export function CommandPalette() {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [entries] = useState(() => buildPaletteEntries());
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const paletteId = useId();

  const filtered = filterPaletteEntries(entries, query);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen(true);
        setQuery("");
        setSelected(0);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    if (open) {
      const focusTimer = setTimeout(() => inputRef.current?.focus(), 0);
      document.body.style.overflow = "hidden";
      return () => {
        clearTimeout(focusTimer);
        document.body.style.overflow = "";
      };
    }
  }, [open]);

  useEffect(() => {
    if (listRef.current && filtered[selected]) {
      const item = listRef.current.querySelector(`[data-index="${selected}"]`);
      item?.scrollIntoView({ block: "nearest" });
    }
  }, [selected, filtered]);

  const handleKeyDown = (e: ReactKeyboardEvent) => {
    if (e.key === "Escape") {
      setOpen(false);
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, filtered.length - 1));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const entry = filtered[selected];
      if (entry) {
        navigate(entry.route);
        setOpen(false);
      }
      return;
    }
  };

  const handleInputChange = (e: ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value);
    setSelected(0);
  };

  if (!open) return null;

  return (
    <div
      className="cmd-overlay"
      onClick={() => setOpen(false)}
      role="dialog"
      aria-modal="true"
      aria-labelledby={paletteId}
    >
      <div className="cmd-panel" onClick={(e) => e.stopPropagation()}>
        <header className="cmd-header">
          <label htmlFor={paletteId} className="cmd-label">
            <Search className="cmd-search-icon" />
            <input
              id={paletteId}
              ref={inputRef}
              type="search"
              value={query}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              placeholder="Search documentation…"
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <kbd className="cmd-shortcut">⌘K</kbd>
        </header>

        <ul className="cmd-list" ref={listRef} role="listbox" aria-label="Results">
          {filtered.length === 0 ? (
            <li className="cmd-empty" role="option" aria-selected="false">
              No matches for "{query}"
            </li>
          ) : (
            filtered.slice(0, 8).map((entry, idx) => (
              <li
                key={entry.route}
                data-index={idx}
                className={`cmd-item ${idx === selected ? "selected" : ""}`}
                role="option"
                aria-selected={idx === selected}
                onClick={() => {
                  navigate(entry.route);
                  setOpen(false);
                }}
                onMouseEnter={() => setSelected(idx)}
              >
                <span className="cmd-item-title">{entry.title}</span>
                <span className="cmd-item-desc">{entry.description}</span>
                <span className="cmd-item-group">{entry.group}</span>
              </li>
            ))
          )}
        </ul>

        <footer className="cmd-footer">
          {filtered.length > 8 && (
            <span className="cmd-more">
              +{filtered.length - 8} more matches
            </span>
          )}
          <span className="cmd-hint">
            <kbd>↑</kbd><kbd>↓</kbd> navigate &nbsp;
            <kbd>Enter</kbd> open &nbsp;
            <kbd>Esc</kbd> close
          </span>
        </footer>
      </div>
    </div>
  );
}

export default CommandPalette;