import { describe, expect, test } from "bun:test";
import { filterPaletteEntries, type PaletteEntry } from "./palette";

const entries: PaletteEntry[] = [
  {
    title: "Getting Started",
    description: "Install and launch OrbyNode.",
    route: "/docs/getting-started",
    group: "Getting started",
    kind: "page",
  },
  {
    title: "Realtime protocol",
    description: "Snapshots, replay, and bounded event queues.",
    route: "/docs/realtime",
    group: "Core concepts",
    kind: "page",
  },
  {
    title: "Overflow recovery",
    description: "Realtime protocol heading",
    route: "/docs/realtime#overflow-recovery",
    group: "Realtime",
    kind: "heading",
  },
];

describe("filterPaletteEntries", () => {
  test("returns a useful default set for an empty query", () => {
    expect(filterPaletteEntries(entries, "")).toEqual(entries);
  });

  test("matches titles, descriptions, and groups without case sensitivity", () => {
    expect(filterPaletteEntries(entries, "BOUNDED").map((entry) => entry.title)).toEqual([
      "Realtime protocol",
    ]);
    expect(filterPaletteEntries(entries, "core concepts").map((entry) => entry.title)).toEqual([
      "Realtime protocol",
    ]);
  });

  test("ranks title-prefix matches before description-only matches", () => {
    expect(filterPaletteEntries(entries, "realtime").map((entry) => entry.title)).toEqual([
      "Realtime protocol",
      "Overflow recovery",
    ]);
  });

  test("normalizes extra whitespace", () => {
    expect(filterPaletteEntries(entries, "  overflow   recovery ").map((entry) => entry.title)).toEqual([
      "Overflow recovery",
    ]);
  });
});
