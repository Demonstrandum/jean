// LISTS: shopping and notes (data/groceries.json and data/notes.json; Claude edits them when asked).
ui.defineScreen({
  id: "lists",
  tab: "單",
  tiles: [
    {
      id: "shopping", at: zone(Z.A1, Z.C3), title: "Shopping", glyph: "bag", tone: "list",
      body: bind(K.shop.items, (items) => (items?.length ? checklist(items.slice(0, 9).map((i) => [i.text, i.qty, i.done])) : div({ style: "font:600 17px var(--sans)" }, "Nothing needed"))),
      count: bind(K.shop.items, (items) => `${(items ?? []).filter((i) => !i.done).length} to get · data/groceries.json`),
    },
    {
      id: "notes", at: zone(Z.D1, Z.F3), title: "Notes", glyph: "flag", tone: "list",
      body: bind(K.notes.items, (items) => (items?.length ? lines(items.slice(0, 9).map((n) => [n.due, n.text, ""])) : div({ style: "font:600 17px var(--sans)" }, "No notes. Ask Claude to remember something."))),
      count: bind(K.notes.items, (items) => `${(items ?? []).length} notes · data/notes.json`),
    },
  ],
});
