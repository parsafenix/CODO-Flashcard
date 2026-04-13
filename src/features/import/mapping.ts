import type { ImportColumnMapping } from "../../lib/types";

export function selectedFieldIds(mappings: ImportColumnMapping[]) {
  return mappings
    .map((mapping) => mapping.field_id ?? null)
    .filter((fieldId): fieldId is number => typeof fieldId === "number");
}

export function isExistingFieldMappingDisabled(
  mappings: ImportColumnMapping[],
  currentColumnIndex: number,
  fieldId: number
) {
  return mappings.some(
    (mapping) => mapping.column_index !== currentColumnIndex && mapping.field_id === fieldId
  );
}
