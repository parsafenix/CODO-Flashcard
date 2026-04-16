type Translate = (key: string, params?: Record<string, string | number>) => string;

const expectedColumnsPattern = /^Expected (\d+) columns but found (\d+)\.$/;
const mapRequiredPattern = /^Map every required deck field before importing: (.+)$/;

export function localizeAppMessage(message: string, t: Translate): string {
  if (!message) {
    return message;
  }

  if (message === "Expected at least 2 columns.") {
    return t("import.invalidReason.minColumns");
  }

  const expectedColumnsMatch = message.match(expectedColumnsPattern);
  if (expectedColumnsMatch) {
    return t("import.invalidReason.columnCount", {
      expected: Number(expectedColumnsMatch[1]),
      found: Number(expectedColumnsMatch[2]),
    });
  }

  if (message === "Matches another card by this deck's active required fields.") {
    return t("import.duplicateReason.existing");
  }

  if (message === "Repeated in this file for the same active required fields.") {
    return t("import.duplicateReason.file");
  }

  if (message === "Each active deck field can only be mapped once.") {
    return t("import.mappingUnique");
  }

  const mapRequiredMatch = message.match(mapRequiredPattern);
  if (mapRequiredMatch) {
    return t("import.mapRequiredFieldsBeforeCommit", { fields: mapRequiredMatch[1] });
  }

  switch (message) {
    case "There is not enough reliable local review data to fit a statistically meaningful parameter update yet.":
      return t("calibration.reason.insufficientData");
    case "Validation log loss did not improve enough.":
      return t("calibration.reason.validationLogLoss");
    case "Validation RMSE (bins) regressed beyond the allowed guardrail.":
      return t("calibration.reason.validationRmse");
    case "Test log loss regressed, so the new fit was not activated.":
      return t("calibration.reason.testLogLoss");
    case "The fitted parameters would increase the 30-day review load too aggressively.":
      return t("calibration.reason.workload");
    case "Validation metrics improved and workload stayed within the safety guardrails.":
      return t("calibration.reason.accepted");
    default:
      return message;
  }
}

export function localizeCalibrationStatus(status: string | null | undefined, t: Translate): string {
  switch (status) {
    case "accepted":
      return t("calibration.status.accepted");
    case "rejected":
      return t("calibration.status.rejected");
    case "insufficient_data":
      return t("calibration.status.insufficientData");
    case null:
    case undefined:
    case "":
      return t("analytics.calibration.notRun");
    default:
      return status;
  }
}

export function localizeCardStatus(status: string, t: Translate): string {
  switch (status) {
    case "new":
      return t("status.new");
    case "learning":
      return t("status.learning");
    case "review":
      return t("status.review");
    case "mastered":
      return t("status.mastered");
    default:
      return status;
  }
}
