// Safe markdown renderer for docs/content/*.md. React nodes only - never
// dangerouslySetInnerHTML. Handles what the docs actually use: headings h1-h4,
// fenced code (with optional language), 4-space indented code, pipe tables with
// alignment rows, ordered/unordered lists with task-list checkboxes, blockquotes,
// hr, paragraphs, and inline parsing (code/bold/italic/links/autolinks). Local
// links are normalized via normalizeMarkdownHref (source-relative) and rendered
// as React Router <Link> for internal targets, safe new-tab anchors for external.
 
import type { MouseEvent, ReactNode } from "react";
import { Link } from "react-router-dom";
import { normalizeMarkdownHref } from "../manifest";

export interface Heading {
  id: string;
  text: string;
  level: 1 | 2 | 3 | 4;
}

/** Slugify heading text: lowercase, non-alphanumerics collapse to "-", trim. */
export function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** Parse a table delimiter row ("| :--- | ---: |") into alignments, or null. */
export function parseAlignments(
  row: string
): ("left" | "center" | "right" | null)[] | null {
  const trimmed = row.trim();
  if (!trimmed) return null;
  const cells = trimmed
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());
  if (cells.length === 0 || cells.some((c) => !/^:?-+:?$/.test(c))) return null;
  return cells.map((c) => {
    const left = c.startsWith(":");
    const right = c.endsWith(":");
    if (left && right) return "center";
    if (right) return "right";
    if (left) return "left";
    return null;
  });
}

function copyCode(event: MouseEvent<HTMLButtonElement>, text: string): void {
  const button = event.currentTarget;
  void navigator.clipboard?.writeText(text).then(
    () => {
      button.classList.add("ok");
      button.textContent = "Copied!";
      button.setAttribute("aria-label", "Copied");
      window.setTimeout(() => {
        button.classList.remove("ok");
        button.textContent = "Copy";
        button.setAttribute("aria-label", "Copy code");
      }, 1400);
    },
    () => undefined,
  );
}

function CodeBlock({ code, lang }: { code: string; lang?: string }) {
  return (
    <div className="md-code">
      <pre data-lang={lang ?? ""}>
        <code className={lang ? `language-${lang}` : undefined}>{code}</code>
      </pre>
      <button
        type="button"
        className="copy-btn"
        aria-label="Copy code"
        onClick={(event) => copyCode(event, code)}
      >
        Copy
      </button>
    </div>
  );
}

/** Resolve a raw markdown href against the doc source into a renderable link. */
function resolveLink(
  href: string,
  source: string
): { internal: boolean; to: string } {
  const normalized = normalizeMarkdownHref(href, source);
  const external =
    /^(https?:)?\/\//.test(normalized) || normalized.startsWith("mailto:");
  return { internal: !external, to: normalized };
}

/** Find the earliest inline construct in `text`, or null. */
function nextInlineMatch(
  text: string,
  source: string
): {
  index: number;
  matched: string;
  render: (key: string) => ReactNode;
} | null {
  let best: {
    index: number;
    matched: string;
    render: (key: string) => ReactNode;
  } | null = null;

  const consider = (
    index: number,
    matched: string,
    render: (key: string) => ReactNode
  ) => {
    if (index >= 0 && (best === null || index < best.index)) {
      best = { index, matched, render };
    }
  };

  // Inline code: `...`
  const codeStart = text.indexOf("`");
  if (codeStart >= 0) {
    const codeEnd = text.indexOf("`", codeStart + 1);
    if (codeEnd > codeStart) {
      const matched = text.slice(codeStart, codeEnd + 1);
      consider(codeStart, matched, (key) => (
        <code key={key} className="md-inline-code">
          {matched.slice(1, -1)}
        </code>
      ));
    }
  }

  // Bold: **...** (non-empty inner)
  const boldStart = text.indexOf("**");
  if (boldStart >= 0) {
    const boldEnd = text.indexOf("**", boldStart + 2);
    if (boldEnd > boldStart + 2) {
      const inner = text.slice(boldStart + 2, boldEnd);
      consider(boldStart, `**${inner}**`, (key) => (
        <strong key={key}>{inner}</strong>
      ));
    }
  }

  // Italic: *...* (single asterisk, non-empty inner)
  const italicStart = text.indexOf("*");
  if (italicStart >= 0 && text[italicStart + 1] !== "*") {
    const italicEnd = text.indexOf("*", italicStart + 1);
    if (italicEnd > italicStart + 1 && text[italicEnd + 1] !== "*") {
      const inner = text.slice(italicStart + 1, italicEnd);
      consider(italicStart, `*${inner}*`, (key) => <em key={key}>{inner}</em>);
    }
  }

  // Link: [text](href)
  const linkStart = text.indexOf("[");
  if (linkStart >= 0) {
    const textEnd = text.indexOf("]", linkStart + 1);
    if (textEnd > linkStart && text[textEnd + 1] === "(") {
      const hrefEnd = text.indexOf(")", textEnd + 2);
      if (hrefEnd > textEnd + 2) {
        const linkText = text.slice(linkStart + 1, textEnd);
        const href = text.slice(textEnd + 2, hrefEnd);
        const matched = text.slice(linkStart, hrefEnd + 1);
        consider(linkStart, matched, (key) => {
          const { internal, to } = resolveLink(href, source);
          return internal ? (
            <Link key={key} to={to}>
              {linkText}
            </Link>
          ) : (
            <a href={to} target="_blank" rel="noopener noreferrer">
              {linkText}
            </a>
          );
        });
      }
    }
  }

  // Autolink: <https?://...>
  const autoMatch = /<(https?:\/\/[^>\s]+)>/.exec(text);
  if (autoMatch) {
    const url = autoMatch[1];
    consider(autoMatch.index, autoMatch[0], (key) => (
      <a key={key} href={url} target="_blank" rel="noopener noreferrer">
        {url}
      </a>
    ));
  }

  return best;
}

/**
 * Parse inline markdown (code, bold, italic, links, autolinks, plain text).
 * Returns interleaved text/element nodes with stable per-callsite keys.
 */
function parseInline(
  text: string,
  keyPrefix: string,
  source: string
): ReactNode[] {
  const nodes: ReactNode[] = [];
  let rest = text;
  let k = 0;
  let guard = 0;
  while (rest.length > 0 && guard++ < 10000) {
    const next = nextInlineMatch(rest, source);
    if (!next) {
      nodes.push(rest);
      break;
    }
    if (next.index > 0) nodes.push(rest.slice(0, next.index));
    nodes.push(next.render(`${keyPrefix}-${k++}`));
    rest = rest.slice(next.index + next.matched.length);
  }
  return nodes;
}

/** Strip inline markup so heading slugs are built from visible text. */
function plainHeadingText(raw: string): string {
  return raw
    .replace(/`([^`]*)`/g, "$1")
    .replace(/\*\*([^*]*)\*\*/g, "$1")
    .replace(/\*([^*]*)\*/g, "$1")
    .replace(/\[([^\]]*)\]\(([^)]*)\)/g, "$1");
}

export function renderMarkdown(
  content: string,
  source: string
): { nodes: ReactNode[]; headings: Heading[] } {
  const lines = content.replace(/\r\n?/g, "\n").split("\n");
  const nodes: ReactNode[] = [];
  const headings: Heading[] = [];
  const idCounts = new Map<string, number>();
  let key = 0;
  let i = 0;

  // Unique, deterministic heading ids: first "x" stays "x", repeats get -1, -2...
  const headingId = (raw: string): string => {
    const plain = plainHeadingText(raw);
    let base = slugify(plain);

    if (!base) base = "heading";
    const count = idCounts.get(base) ?? 0;
    idCounts.set(base, count + 1);
    return count === 0 ? base : `${base}-${count}`;
  };

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    // Fenced code (``` with optional language)
    if (line.startsWith("```")) {
      const lang = line.slice(3).trim();
      const body: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) {
        body.push(lines[i]);
        i++;
      }
      if (i < lines.length) i++; // consume closing fence
      nodes.push(
        <CodeBlock key={key++} code={body.join("\n")} lang={lang || undefined} />
      );
      continue;
    }

    // 4-space indented code (repo convention).
    if (/^ {4}\S/.test(line)) {
      const body: string[] = [];
      while (i < lines.length && /^ {4}/.test(lines[i])) {
        body.push(lines[i].slice(4));
        i++;
      }
      while (body.length > 0 && body[body.length - 1] === "") body.pop();
      if (body.length > 0) {
        nodes.push(<CodeBlock key={key++} code={body.join("\n")} />);
      }
      continue;
    }

    // Headings
    const headingMatch = line.match(/^(#{1,4})\s+(.+?)\s*#*\s*$/);
    if (headingMatch) {
      const level = headingMatch[1].length as 1 | 2 | 3 | 4;
      const text = headingMatch[2];
      const id = headingId(text);
      headings.push({ id, text, level });
      const Tag = `h${level}` as "h1" | "h2" | "h3" | "h4";
      nodes.push(
        <Tag key={key++} id={id}>
          {parseInline(text, `h-${key}`, source)}
        </Tag>
      );
      i++;
      continue;
    }

    // Pipe table: header row with pipes + next line is a delimiter row.
    if (
      trimmed.includes("|") &&
      i + 1 < lines.length &&
      parseAlignments(lines[i + 1]) !== null
    ) {
      const aligns = parseAlignments(lines[i + 1]);
      const splitRow = (row: string): string[] =>
        row
          .trim()
          .replace(/^\|/, "")
          .replace(/\|$/, "")
          .split("|")
          .map((c) => c.trim());
      const header = splitRow(line);
      i += 2;
      const bodyRows: string[][] = [];
      while (i < lines.length && lines[i].includes("|") && lines[i].trim() !== "") {
        bodyRows.push(splitRow(lines[i]));
        i++;
      }
      const alignList = aligns;
      const cellStyle = (n: number) =>
        alignList && alignList[n] ? { textAlign: alignList[n] } : undefined;
      nodes.push(
        <div key={key++} className="md-table-wrap">
          <table>
            <thead>
              <tr>
                {header.map((cell, n) => (
                  <th key={n} style={cellStyle(n)}>
                    {parseInline(cell, `th-${key}-${n}`, source)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {bodyRows.map((row, r) => (
                <tr key={r}>
                  {header.map((_, c) => {
                    const cell = row[c] ?? "";
                    return (
                      <td key={c} style={cellStyle(c)}>
                        {parseInline(cell, `td-${key}-${r}-${c}`, source)}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
      continue;
    }

    // Unordered list (with optional task-list checkboxes).
    if (/^[-*+]\s+/.test(line)) {
      const items: { text: string; task: boolean; done: boolean }[] = [];
      while (i < lines.length && /^[-*+]\s+/.test(lines[i])) {
        let item = lines[i].replace(/^[-*+]\s+/, "");
        let task = false;
        let done = false;
        const taskMatch = /^\[([ xX])\]\s+/.exec(item);
        if (taskMatch) {
          task = true;
          done = taskMatch[1].toLowerCase() === "x";
          item = item.slice(taskMatch[0].length);
        }
        items.push({ text: item, task, done });
        i++;
      }
      if (items.some((it) => it.task)) {
        nodes.push(
          <ul key={key++} className="md-task-list">
            {items.map((it, n) => (
              <li key={n}>
                {it.task ? [
                  <input key="cb" type="checkbox" checked={it.done} readOnly />,
                  <span key="txt">
                    {parseInline(it.text, `task-${key}-${n}`, source)}
                  </span>
                ] : (
                  <span>{parseInline(it.text, `li-${key}-${n}`, source)}</span>
                )}
              </li>
            ))}
          </ul>
        );
      } else {
        nodes.push(
          <ul key={key++}>
            {items.map((it, n) => (
              <li key={n}>{parseInline(it.text, `li-${key}-${n}`, source)}</li>
            ))}
          </ul>
        );
      }
      continue;
    }

    // Ordered list
    if (/^\d+[.)]\s+/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+[.)]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+[.)]\s+/, ""));
        i++;
      }
      nodes.push(
        <ol key={key++}>
          {items.map((it, n) => (
            <li key={n}>{parseInline(it, `oli-${key}-${n}`, source)}</li>
          ))}
        </ol>
      );
      continue;
    }

    // Blockquote: consecutive "> " lines merge; blank "> " splits paragraphs.
    if (line.startsWith(">")) {
      const quoteLines: string[] = [];
      while (i < lines.length && lines[i].startsWith(">")) {
        quoteLines.push(lines[i].replace(/^> ?/, ""));
        i++;
      }
      const quoteNodes: ReactNode[] = [];
      let para: string[] = [];
      const flush = () => {
        if (para.length > 0) {
          quoteNodes.push(
            <p key={`qp-${quoteNodes.length}`}>
              {parseInline(
                para.join(" "),
                `bq-${key}-${quoteNodes.length}`,
                source
              )}
            </p>
          );
          para = [];
        }
      };
      for (const ql of quoteLines) {
        if (ql.trim() === "") flush();
        else para.push(ql);
      }
      flush();
      nodes.push(<blockquote key={key++}>{quoteNodes}</blockquote>);
      continue;
    }

    // Horizontal rule
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      nodes.push(<hr key={key++} />);
      i++;
      continue;
    }

    // Blank line: spacing handled by CSS margins.
    if (trimmed === "") {
      i++;
      continue;
    }

    // Paragraph: gather consecutive lines that don't start a block construct.
    const paraLines: string[] = [line];
    i++;
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !/^#{1,4}\s/.test(lines[i]) &&
      !lines[i].startsWith("```") &&
      !/^ {4}\S/.test(lines[i]) &&
      !/^[-*+]\s+/.test(lines[i]) &&
      !/^\d+[.)]\s+/.test(lines[i]) &&
      !lines[i].startsWith(">") &&
      !/^(-{3,}|\*{3,}|_{3,})$/.test(lines[i].trim())
    ) {
      paraLines.push(lines[i]);
      i++;
    }
    nodes.push(
      <p key={key++}>{parseInline(paraLines.join(" "), `p-${key}`, source)}</p>
    );
  }

  return { nodes, headings };
}

/** React component: render markdown content sourced from `source`. */
export function Markdown({
  content,
  source = "",
}: {
  content: string;
  source?: string;
}) {
  const { nodes } = renderMarkdown(content, source);
  return <>{nodes}</>;
}

export default Markdown;
