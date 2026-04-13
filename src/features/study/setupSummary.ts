import type { DeckField } from "../../lib/types";
import { activeFieldLabel } from "../../lib/deckFields";

export interface StudyDirectionPreview {
  front: string;
  reveal: string[];
}

export function buildStudyDirectionPreview(
  fields: DeckField[],
  promptFieldId: number,
  revealFieldIds: number[],
  reverseMode: boolean
): StudyDirectionPreview {
  if (!reverseMode || revealFieldIds.length === 0) {
    return {
      front: activeFieldLabel(fields, promptFieldId),
      reveal: revealFieldIds.map((fieldId) => activeFieldLabel(fields, fieldId)),
    };
  }

  const [firstReveal, ...restReveal] = revealFieldIds;
  return {
    front: activeFieldLabel(fields, firstReveal),
    reveal: [activeFieldLabel(fields, promptFieldId), ...restReveal.map((fieldId) => activeFieldLabel(fields, fieldId))],
  };
}
