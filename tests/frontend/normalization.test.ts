import { compactSearchText, normalizeSearchText } from "../../src/lib/normalization";

describe("normalizeSearchText", () => {
  it("normalizes Persian Arabic characters and whitespace", () => {
    expect(normalizeSearchText("  كتاب\u200cها  ")).toBe("کتاب ها");
    expect(compactSearchText("  كتاب\u200cها  ")).toBe("کتابها");
  });

  it("handles mixed latin casing consistently", () => {
    expect(normalizeSearchText("  HeLLo   World  ")).toBe("hello world");
  });
});

