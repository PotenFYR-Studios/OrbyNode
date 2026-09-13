import { describe, test, expect } from "bun:test";
import type { ReactElement } from "react";
import { Link } from "react-router-dom";
import { slugify, parseAlignments, renderMarkdown, type Heading } from "./Markdown";

const el = (node: unknown) => node as ReactElement;

/** Evaluate one level of function components (CodeBlock) to its output. */
function rendered(node: unknown): ReactElement {
  const element = el(node);
  if (typeof element.type !== "function") return element;
  const type = element.type as (props: unknown) => ReactElement;
  return el(type(element.props));
}

describe("slugify", () => {
  test("lowercases text", () => {
    expect(slugify("Hello World")).toBe("hello-world");
  });

  test("replaces non-alphanumeric with hyphens", () => {
    expect(slugify("foo bar")).toBe("foo-bar");
    expect(slugify("foo   bar")).toBe("foo-bar");
    expect(slugify("foo_bar")).toBe("foo-bar");
    expect(slugify("foo!bar")).toBe("foo-bar");
  });

  test("removes leading/trailing hyphens", () => {
    expect(slugify("-foo-")).toBe("foo");
    expect(slugify("---foo---")).toBe("foo");
  });

  test("handles empty string", () => {
    expect(slugify("")).toBe("");
  });

  test("handles special characters only", () => {
    expect(slugify("!@#$%")).toBe("");
  });

  test("preserves numbers", () => {
    expect(slugify("Section 123")).toBe("section-123");
  });

  test("handles consecutive spaces", () => {
    expect(slugify("foo    bar    baz")).toBe("foo-bar-baz");
  });
});

describe("parseAlignments", () => {
  test("parses alignment row", () => {
    expect(parseAlignments("| :--- | :---: | ---: | --- |")).toEqual([
      "left",
      "center",
      "right",
      null,
    ]);
  });

  test("parses without pipes at edges", () => {
    expect(parseAlignments("--- | :---")).toEqual([null, "left"]);
  });

  test("returns null for non-delimiter rows", () => {
    expect(parseAlignments("| a | b |")).toBeNull();
    expect(parseAlignments("| -a- | b |")).toBeNull();
    expect(parseAlignments("")).toBeNull();
  });
});

describe("renderMarkdown headings", () => {
  test("collects h1-h4 with deterministic ids", () => {
    const { headings } = renderMarkdown(
      ["# Top", "## Alpha", "### Beta", "#### Gamma", "## Delta"].join("\n"),
      "docs/intro.md"
    );
    expect(
      headings.map((h: Heading) => [h.level, h.id, h.text])
    ).toEqual([
      [1, "top", "Top"],
      [2, "alpha", "Alpha"],
      [3, "beta", "Beta"],
      [4, "gamma", "Gamma"],
      [2, "delta", "Delta"],
    ]);
  });

  test("deduplicates heading ids deterministically", () => {
    const { headings } = renderMarkdown("## Setup\n## Setup\n## Setup", "docs/intro.md");
    expect(headings.map((h) => h.id)).toEqual(["setup", "setup-1", "setup-2"]);
  });

  test("fallback id when heading has no slug chars", () => {
    const { headings } = renderMarkdown("## ???\n## ???", "docs/intro.md");
    expect(headings[0]!.id).toBe("heading");
    expect(headings[1]!.id).toBe("heading-1");
  });

  test("renders heading elements with matching ids", () => {
    const { nodes } = renderMarkdown("## Hello", "docs/intro.md");
    const h2 = el(nodes[0]);
    expect(h2.type).toBe("h2");
    expect(h2.props.id).toBe("hello");
  });
});

describe("renderMarkdown tables", () => {
  test("parses pipe table with alignment row", () => {
    const { nodes } = renderMarkdown(
      "| A | B |\n| :-- | --: |\n| 1 | 2 |",
      "docs/intro.md"
    );
    const wrap = el(nodes[0]); // <div className="md-table-wrap">
    const table = el(wrap.props.children); // <table>
    const thead = el(table.props.children[0]); // <thead>
    const headRow = el(thead.props.children); // <tr>
    const th0 = el(headRow.props.children[0]); // <th>
    expect(th0.type).toBe("th");
    expect(th0.props.style).toEqual({ textAlign: "left" });
  });

  test("alignment row is not rendered as a body row", () => {
    const { nodes } = renderMarkdown(
      "| A | B |\n| -- | -- |\n| 1 | 2 |",
      "docs/intro.md"
    );
    const wrap = rendered(nodes[0]);
    const table = el(wrap.props.children);
    const tbody = el(table.props.children[1]); // <tbody>
    expect(tbody.props.children).toHaveLength(1);
  });

  test("pipe-containing prose is not a table without delimiter row", () => {
    const { nodes } = renderMarkdown("a | b\nc | d", "docs/intro.md");
    const p = el(nodes[0]);
    expect(p.type).toBe("p");
  });
});

describe("renderMarkdown links", () => {
  test("normalizes relative doc links to router Link", () => {
    const { nodes } = renderMarkdown("[install](./installation.md)", "docs/intro.md");
    const p = el(nodes[0]);
    const a = el(p.props.children[0]);
    expect(a.type).toBe(Link);
    expect(a.props.to).toBe("/docs/installation");
  });

  test("preserves hash fragments on normalized links", () => {
    const { nodes } = renderMarkdown(
      "[part](./installation.md#install)",
      "docs/intro.md"
    );
    const p = el(nodes[0]);
    const a = el(p.props.children[0]);
    expect(a.type).toBe(Link);
    expect(a.props.to).toBe("/docs/installation#install");
  });

  test("hash-only links stay on page via Link", () => {
    const { nodes } = renderMarkdown("[jump](#section)", "docs/intro.md");
    const p = el(nodes[0]);
    const a = el(p.props.children[0]);
    expect(a.type).toBe(Link);
    expect(a.props.to).toBe("#section");
  });

  test("absolute site links render as Link", () => {
    const { nodes } = renderMarkdown("[docs](/docs)", "docs/intro.md");
    const p = el(nodes[0]);
    const a = el(p.props.children[0]);
    expect(a.type).toBe(Link);
    expect(a.props.to).toBe("/docs");
  });

  test("external links open safely in new tab", () => {
    const { nodes } = renderMarkdown(
      "[site](https://example.com/x?q=1)",
      "docs/intro.md"
    );
    const p = el(nodes[0]);
    const a = el(p.props.children[0]);
    expect(a.type).toBe("a");
    expect(a.props.href).toBe("https://example.com/x?q=1");
    expect(a.props.target).toBe("_blank");
    expect(a.props.rel).toBe("noopener noreferrer");
  });

  test("autolinks render as external anchors", () => {
    const { nodes } = renderMarkdown("see <https://example.com>", "docs/intro.md");
    const p = el(nodes[0]);
    // "see " is text, then <a> element
    const a = el(p.props.children[1]);
    expect(a.type).toBe("a");
    expect(a.props.href).toBe("https://example.com");
    expect(a.props.target).toBe("_blank");
    expect(a.props.rel).toBe("noopener noreferrer");
  });
});

describe("renderMarkdown blocks", () => {
  test("fenced code with language and copy button", () => {
    const { nodes } = renderMarkdown(
      "```ts\nconst x = 1;\n```",
      "docs/intro.md"
    );
    const wrap = rendered(nodes[0]);
    const pre = el(wrap.props.children[0]);
    expect(pre.type).toBe("pre");
    const code = el(pre.props.children);
    expect(code.type).toBe("code");
    expect(code.props.className).toContain("language-ts");
    expect(code.props.children).toBe("const x = 1;");
    const btn = el(wrap.props.children[1]);
    expect(btn.type).toBe("button");
  });

  test("4-space indented code becomes a code block", () => {
    const { nodes } = renderMarkdown("    indented()", "docs/intro.md");
    const wrap = rendered(nodes[0]);
    const pre = el(wrap.props.children[0]);
    expect(pre.type).toBe("pre");
    const code = el(pre.props.children);
    expect(code.props.children).toBe("indented()");
  });

  test("multi-line fenced code preserves newlines", () => {
    const { nodes } = renderMarkdown("```\nline1\nline2\n```", "docs/intro.md");
    const wrap = rendered(nodes[0]);
    const code = el(el(wrap.props.children[0]).props.children);
    expect(code.props.children).toBe("line1\nline2");
  });

  test("unordered list", () => {
    const { nodes } = renderMarkdown("- one\n- two", "docs/intro.md");
    const ul = el(nodes[0]);
    expect(ul.type).toBe("ul");
    expect(ul.props.children).toHaveLength(2);
  });

  test("ordered list", () => {
    const { nodes } = renderMarkdown("1. one\n2. two", "docs/intro.md");
    const ol = el(nodes[0]);
    expect(ol.type).toBe("ol");
    expect(ol.props.children).toHaveLength(2);
  });

  test("task list renders checkboxes", () => {
    const { nodes } = renderMarkdown(
      "- [x] done\n- [ ] pending",
      "docs/intro.md"
    );
    const ul = el(nodes[0]);
    const first = el(ul.props.children[0]);
    const input = el(first.props.children[0]);
    expect(input.type).toBe("input");
    expect(input.props.checked).toBe(true);
    expect(input.props.readOnly).toBe(true);
    const second = el(ul.props.children[1]);
    const input2 = el(second.props.children[0]);
    expect(input2.props.checked).toBe(false);
  });

  test("blockquote", () => {
    const { nodes } = renderMarkdown("> quoted", "docs/intro.md");
    expect(el(nodes[0]).type).toBe("blockquote");
  });

  test("hr", () => {
    const { nodes } = renderMarkdown("---", "docs/intro.md");
    expect(el(nodes[0]).type).toBe("hr");
  });

  test("blank lines produce no nodes", () => {
    const { nodes } = renderMarkdown("a\n\n\nb", "docs/intro.md");
    expect(nodes).toHaveLength(2);
  });

  test("soft-wrapped paragraph lines join", () => {
    const { nodes } = renderMarkdown("one two\nthree four", "docs/intro.md");
    expect(nodes).toHaveLength(1);
    const p = el(nodes[0]);
    expect(p.props.children).toContain("one two three four");
  });
});

describe("renderMarkdown inline", () => {
  test("inline code, bold, italic", () => {
    const { nodes } = renderMarkdown("a `code` b **bold** c *it*", "docs/intro.md");
    const p = el(nodes[0]);
    const kids = p.props.children as ReactElement[];
    expect(kids.some((k) => k.type === "code")).toBe(true);
    expect(kids.some((k) => k.type === "strong")).toBe(true);
    expect(kids.some((k) => k.type === "em")).toBe(true);
  });

  test("bold inside heading still yields plain text id", () => {
    const { headings } = renderMarkdown("## The **Big** Idea", "docs/intro.md");
    expect(headings[0]!.id).toBe("the-big-idea");
  });
});
