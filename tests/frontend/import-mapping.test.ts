import { isExistingFieldMappingDisabled } from "../../src/features/import/mapping";

describe("isExistingFieldMappingDisabled", () => {
  it("prevents reusing the same existing field for another column", () => {
    const mappings = [
      { column_index: 0, field_id: 12 },
      { column_index: 1, field_id: 18 },
    ];

    expect(isExistingFieldMappingDisabled(mappings, 1, 12)).toBe(true);
    expect(isExistingFieldMappingDisabled(mappings, 0, 12)).toBe(false);
    expect(isExistingFieldMappingDisabled(mappings, 1, 99)).toBe(false);
  });
});
