import { describe, expect, it } from "vitest";
import { localizeAppMessage, localizeCalibrationStatus, localizeCardStatus } from "./messages";

function buildTranslator() {
  return (key: string, params?: Record<string, string | number>) => {
    if (!params) {
      return key;
    }

    return `${key}:${Object.entries(params)
      .map(([name, value]) => `${name}=${value}`)
      .join(",")}`;
  };
}

describe("messages localization helpers", () => {
  const t = buildTranslator();

  it("maps import parser and duplicate messages to i18n keys", () => {
    expect(localizeAppMessage("Expected at least 2 columns.", t)).toBe("import.invalidReason.minColumns");
    expect(localizeAppMessage("Expected 4 columns but found 2.", t)).toBe(
      "import.invalidReason.columnCount:expected=4,found=2"
    );
    expect(localizeAppMessage("Matches another card by this deck's active required fields.", t)).toBe(
      "import.duplicateReason.existing"
    );
    expect(localizeAppMessage("Repeated in this file for the same active required fields.", t)).toBe(
      "import.duplicateReason.file"
    );
    expect(localizeAppMessage("Each active deck field can only be mapped once.", t)).toBe("import.mappingUnique");
    expect(localizeAppMessage("Map every required deck field before importing: Front, Back", t)).toBe(
      "import.mapRequiredFieldsBeforeCommit:fields=Front, Back"
    );
  });

  it("maps calibration reasons and statuses", () => {
    expect(
      localizeAppMessage(
        "There is not enough reliable local review data to fit a statistically meaningful parameter update yet.",
        t
      )
    ).toBe("calibration.reason.insufficientData");
    expect(localizeCalibrationStatus("accepted", t)).toBe("calibration.status.accepted");
    expect(localizeCalibrationStatus("rejected", t)).toBe("calibration.status.rejected");
    expect(localizeCalibrationStatus("insufficient_data", t)).toBe("calibration.status.insufficientData");
    expect(localizeCalibrationStatus("", t)).toBe("analytics.calibration.notRun");
  });

  it("maps card statuses for deck tables", () => {
    expect(localizeCardStatus("new", t)).toBe("status.new");
    expect(localizeCardStatus("learning", t)).toBe("status.learning");
    expect(localizeCardStatus("review", t)).toBe("status.review");
    expect(localizeCardStatus("mastered", t)).toBe("status.mastered");
    expect(localizeCardStatus("custom", t)).toBe("custom");
  });
});
