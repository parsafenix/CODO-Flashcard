import { sortDecks } from "../../src/features/decks/sorting";
import type { DeckSummary } from "../../src/lib/types";

function deck(overrides: Partial<DeckSummary>): DeckSummary {
  return {
    id: 1,
    name: "Deck",
    description: null,
    language_1_label: "Persian",
    language_2_label: "English",
    language_3_label: "Italian",
    created_at: "2026-04-01T00:00:00Z",
    updated_at: "2026-04-01T00:00:00Z",
    last_studied_at: null,
    total_cards: 0,
    due_cards: 0,
    new_cards: 0,
    mastered_cards: 0,
    ...overrides
  };
}

describe("sortDecks", () => {
  it("prioritizes decks with more due cards by default sort", () => {
    const result = sortDecks(
      [
        deck({ id: 1, name: "A", due_cards: 2 }),
        deck({ id: 2, name: "B", due_cards: 7 }),
        deck({ id: 3, name: "C", due_cards: 1 })
      ],
      "due_desc"
    );

    expect(result.map((item) => item.id)).toEqual([2, 1, 3]);
  });

  it("sorts alphabetically in ascending order", () => {
    const result = sortDecks(
      [
        deck({ id: 1, name: "Zeta" }),
        deck({ id: 2, name: "Alpha" }),
        deck({ id: 3, name: "Beta" })
      ],
      "name_asc"
    );

    expect(result.map((item) => item.name)).toEqual(["Alpha", "Beta", "Zeta"]);
  });
});
