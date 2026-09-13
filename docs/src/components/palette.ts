export interface PaletteEntry {
  title: string;
  description: string;
  route: string;
  group: string;
  kind: "page" | "heading";
}

/** Score and filter palette entries against a query. */
export function filterPaletteEntries(
  entries: PaletteEntry[],
  query: string,
): PaletteEntry[] {
  const q = query.trim().toLowerCase().replace(/\s+/g, " ");
  if (!q) return entries;

  const scored: { entry: PaletteEntry; score: number }[] = [];
  for (const entry of entries) {
    const title = entry.title.toLowerCase();
    const description = entry.description.toLowerCase();
    const group = entry.group.toLowerCase();

    if (title.startsWith(q)) {
      scored.push({ entry, score: 0 });
    } else if (title.includes(q)) {
      scored.push({ entry, score: 1 });
    } else if (description.includes(q)) {
      scored.push({ entry, score: 2 });
    } else if (group.includes(q)) {
      scored.push({ entry, score: 3 });
    }
  }

  scored.sort((a, b) => a.score - b.score);
  return scored.map(({ entry }) => entry);
}
