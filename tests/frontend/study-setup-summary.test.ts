import { buildStudyDirectionPreview } from "../../src/features/study/setupSummary";
import type { DeckField } from "../../src/lib/types";

const fields: DeckField[] = [
  { id: 1, deck_id: 1, label: "German", language_code: "german", order_index: 0, required: true, active: true, field_type: "text", system_key: null },
  { id: 2, deck_id: 1, label: "English", language_code: "english", order_index: 1, required: true, active: true, field_type: "text", system_key: null },
  { id: 3, deck_id: 1, label: "Example", language_code: "example", order_index: 2, required: false, active: true, field_type: "text", system_key: null },
];

describe("buildStudyDirectionPreview", () => {
  it("shows the chosen prompt and reveal fields in normal mode", () => {
    expect(buildStudyDirectionPreview(fields, 1, [2, 3], false)).toEqual({
      front: "German",
      reveal: ["English", "Example"],
    });
  });

  it("switches the front to the first reveal field in reverse mode", () => {
    expect(buildStudyDirectionPreview(fields, 1, [2, 3], true)).toEqual({
      front: "English",
      reveal: ["German", "Example"],
    });
  });
});
