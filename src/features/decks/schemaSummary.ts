import type { DeckFieldInput } from "../../lib/types";

export interface DeckSchemaSummary {
  activeCount: number;
  requiredCount: number;
  promptEligibleCount: number;
  revealEligibleCount: number;
}

export function summarizeDeckFields(fields: DeckFieldInput[]): DeckSchemaSummary {
  const activeFields = fields.filter((field) => field.active);
  const requiredActiveFields = activeFields.filter((field) => field.required);

  return {
    activeCount: activeFields.length,
    requiredCount: requiredActiveFields.length,
    promptEligibleCount: activeFields.length,
    revealEligibleCount: Math.max(activeFields.length - 1, 0),
  };
}
