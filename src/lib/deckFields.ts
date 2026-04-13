import type { CardValueRecord, DeckField, StudyCard } from "./types";

export function getActiveFields(fields: DeckField[]) {
  return [...fields]
    .filter((field) => field.active)
    .sort((left, right) => left.order_index - right.order_index || left.id - right.id);
}

export function getRequiredActiveFields(fields: DeckField[]) {
  return getActiveFields(fields).filter((field) => field.required);
}

export function getCardValue(cardValues: CardValueRecord[], fieldId: number) {
  return cardValues.find((value) => value.field_id === fieldId)?.value ?? "";
}

export function getStudyFieldValue(card: StudyCard, fieldId: number) {
  return getCardValue(card.values, fieldId);
}

export function defaultPromptFieldId(fields: DeckField[]) {
  return getActiveFields(fields)[0]?.id ?? 0;
}

export function defaultRevealFieldIds(fields: DeckField[], promptFieldId: number) {
  return getActiveFields(fields)
    .filter((field) => field.id !== promptFieldId)
    .map((field) => field.id);
}

export function activeFieldLabel(fields: DeckField[], fieldId: number) {
  return fields.find((field) => field.id === fieldId)?.label ?? "Field";
}

export function isContextField(field: Pick<DeckField, "label" | "language_code">) {
  const code = (field.language_code ?? "").toLowerCase();
  const label = field.label.toLowerCase();
  return ["example", "notes", "note", "definition"].some(
    (token) => code.includes(token) || label.includes(token)
  );
}
