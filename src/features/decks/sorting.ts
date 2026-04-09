import type { DeckLibrarySort, DeckSummary } from "../../lib/types";

function compareNullableDates(left: string | null, right: string | null): number {
  const leftTime = left ? new Date(left).getTime() : 0;
  const rightTime = right ? new Date(right).getTime() : 0;
  return rightTime - leftTime;
}

export function sortDecks(decks: DeckSummary[], sort: DeckLibrarySort): DeckSummary[] {
  const sorted = [...decks];

  sorted.sort((left, right) => {
    switch (sort) {
      case "name_asc":
        return left.name.localeCompare(right.name);
      case "name_desc":
        return right.name.localeCompare(left.name);
      case "recent_studied":
        return compareNullableDates(left.last_studied_at, right.last_studied_at);
      case "new_desc":
        return right.new_cards - left.new_cards || compareNullableDates(left.last_studied_at, right.last_studied_at);
      case "total_desc":
        return right.total_cards - left.total_cards || compareNullableDates(left.last_studied_at, right.last_studied_at);
      case "mastered_desc":
        return right.mastered_cards - left.mastered_cards || compareNullableDates(left.last_studied_at, right.last_studied_at);
      case "created_desc":
        return compareNullableDates(left.created_at, right.created_at);
      case "due_desc":
      default:
        return (
          right.due_cards - left.due_cards ||
          compareNullableDates(left.last_studied_at, right.last_studied_at) ||
          right.new_cards - left.new_cards
        );
    }
  });

  return sorted;
}
