import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { FieldText } from "../../components/ui/FieldText";
import { Modal } from "../../components/ui/Modal";
import { getActiveFields } from "../../lib/deckFields";
import { useI18n } from "../../lib/i18n";
import { api } from "../../lib/api";
import type {
  CommitImportRequest,
  DeckSummary,
  ImportColumnMapping,
  ImportPreviewRequest,
  ImportPreviewResponse,
} from "../../lib/types";
import { ImportHelpModal } from "./ImportHelpModal";
import { isExistingFieldMappingDisabled } from "./mapping";

interface ImportWizardProps {
  open: boolean;
  decks: DeckSummary[];
  fixedDeck?: DeckSummary | null;
  defaultDelimiter: string;
  onClose: () => void;
  onImported: (deckId: number) => Promise<void> | void;
}

export function ImportWizard({ open: isOpen, decks, fixedDeck, defaultDelimiter, onClose, onImported }: ImportWizardProps) {
  const { settings } = useAppContext();
  const { t } = useI18n();
  const [filePath, setFilePath] = useState("");
  const [delimiter, setDelimiter] = useState(defaultDelimiter || "|");
  const [hasHeader, setHasHeader] = useState(false);
  const [createFieldsFromHeader, setCreateFieldsFromHeader] = useState(true);
  const [targetMode, setTargetMode] = useState<"existing" | "new">(fixedDeck ? "existing" : "new");
  const [existingDeckId, setExistingDeckId] = useState<number>(fixedDeck?.id ?? decks[0]?.id ?? 0);
  const [newDeckName, setNewDeckName] = useState("");
  const [newDeckDescription, setNewDeckDescription] = useState("");
  const [mappings, setMappings] = useState<ImportColumnMapping[]>([]);
  const [preview, setPreview] = useState<ImportPreviewResponse | null>(null);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [committing, setCommitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showHelp, setShowHelp] = useState(false);

  const selectedDeck = useMemo(
    () => fixedDeck ?? decks.find((deck) => deck.id === existingDeckId) ?? null,
    [decks, existingDeckId, fixedDeck]
  );
  const selectedDeckFields = useMemo(() => (selectedDeck ? getActiveFields(selectedDeck.fields) : []), [selectedDeck]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setDelimiter(defaultDelimiter || "|");
    setTargetMode(fixedDeck ? "existing" : "new");
    setExistingDeckId(fixedDeck?.id ?? decks[0]?.id ?? 0);
    setPreview(null);
    setMappings([]);
    setFilePath("");
    setNewDeckName("");
    setNewDeckDescription("");
    setHasHeader(false);
    setCreateFieldsFromHeader(true);
    setError(null);
  }, [defaultDelimiter, decks, fixedDeck, isOpen]);

  async function chooseFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Text files", extensions: ["txt"] }],
    });

    if (typeof selected === "string") {
      setFilePath(selected);
      if (!newDeckName) {
        const filename = selected.split(/[\\/]/).pop()?.replace(/\.[^.]+$/i, "") ?? "";
        setNewDeckName(filename);
      }
    }
  }

  function buildPreviewRequest(): ImportPreviewRequest {
    return {
      file_path: filePath,
      delimiter,
      has_header: hasHeader,
      create_fields_from_header: createFieldsFromHeader,
      mappings,
      target:
        targetMode === "existing"
          ? { mode: "existing", deck_id: fixedDeck?.id ?? existingDeckId }
          : {
              mode: "new",
              name: newDeckName,
              description: newDeckDescription,
            },
    };
  }

  function initializeMappings(response: ImportPreviewResponse) {
    if (targetMode === "existing") {
      setMappings(
        response.detected_columns.map((column, index) => ({
          column_index: column.column_index,
          field_id: selectedDeckFields[index]?.id ?? null,
        }))
      );
      return;
    }

    setMappings(
      response.suggested_new_fields.map((field, index) => ({
        column_index: index,
        label: field.label,
        language_code: field.language_code ?? null,
        required: field.required,
        active: field.active,
      }))
    );
  }

  function updateExistingFieldMapping(columnIndex: number, nextFieldId: number | null) {
    setMappings((current) =>
      current.map((mapping) =>
        mapping.column_index === columnIndex
          ? {
              ...mapping,
              field_id: nextFieldId,
            }
          : mapping
      )
    );
  }

  function updateNewFieldMapping(columnIndex: number, patch: Partial<ImportColumnMapping>) {
    setMappings((current) =>
      current.map((mapping) =>
        mapping.column_index === columnIndex
          ? {
              ...mapping,
              ...patch,
            }
          : mapping
      )
    );
  }

  async function handlePreview() {
    if (!filePath) {
      setError(t("import.previewErrorFile"));
      return;
    }

    if (!delimiter.trim()) {
      setError(t("import.previewErrorDelimiter"));
      return;
    }

    if (targetMode === "new" && !newDeckName.trim()) {
      setError(t("import.previewErrorDeckName"));
      return;
    }

    setLoadingPreview(true);
    setError(null);
    try {
      const response = await api.previewImport(buildPreviewRequest());
      setPreview(response);
      if (mappings.length === 0) {
        initializeMappings(response);
      }
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : t("import.previewFailed");
      setError(message);
    } finally {
      setLoadingPreview(false);
    }
  }

  async function handleCommit() {
    if (!preview) {
      return;
    }

    setCommitting(true);
    setError(null);
    try {
      const request: CommitImportRequest = buildPreviewRequest();
      const result = await api.commitImport(request);
      await onImported(result.deck_id);
      onClose();
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : t("import.commitFailed");
      setError(message);
    } finally {
      setCommitting(false);
    }
  }

  return (
    <Modal
      open={isOpen}
      title={t("import.title")}
      description={t("import.description")}
      onClose={onClose}
      width="large"
    >
      <div className="form-stack">
        <div className="surface-muted">
          <div className="surface-muted__label">{t("import.fileSection")}</div>
        </div>

        <div className="field-row">
          <label className="field field--grow">
            <span>{t("import.fileLabel")}</span>
            <input value={filePath} readOnly placeholder={t("import.filePlaceholder")} />
          </label>
          <Button variant="secondary" onClick={() => void chooseFile()}>
            {t("import.chooseFile")}
          </Button>
        </div>

        <div className="field-grid field-grid--triple">
          <label className="field">
            <span>{t("import.delimiter")}</span>
            <input value={delimiter} onChange={(event) => setDelimiter(event.target.value)} maxLength={3} />
          </label>
          <label className="field field--checkbox">
            <input type="checkbox" checked={hasHeader} onChange={(event) => setHasHeader(event.target.checked)} />
            <span>{t("import.firstRowHeader")}</span>
          </label>
          {!fixedDeck ? (
            <label className="field">
              <span>{t("import.target")}</span>
              <select value={targetMode} onChange={(event) => setTargetMode(event.target.value as "existing" | "new")}>
                <option value="new">{t("import.newDeck")}</option>
                <option value="existing">{t("import.existingDeck")}</option>
              </select>
            </label>
          ) : (
            <div className="field">
              <span>{t("import.target")}</span>
              <div className="field-readonly">{fixedDeck.name}</div>
            </div>
          )}
        </div>

        {targetMode === "existing" ? (
          <label className="field">
            <span>{t("import.existingDeckLabel")}</span>
            <select value={fixedDeck?.id ?? existingDeckId} onChange={(event) => setExistingDeckId(Number(event.target.value))}>
              {decks.map((deck) => (
                <option key={deck.id} value={deck.id}>
                  {deck.name}
                </option>
              ))}
            </select>
          </label>
        ) : (
          <>
            <div className="field-grid field-grid--dual">
              <label className="field">
                <span>{t("import.newDeckName")}</span>
                <input dir="auto" value={newDeckName} onChange={(event) => setNewDeckName(event.target.value)} />
              </label>
              <label className="field">
                <span>{t("common.description")}</span>
                <input dir="auto" value={newDeckDescription} onChange={(event) => setNewDeckDescription(event.target.value)} />
              </label>
            </div>
            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={createFieldsFromHeader}
                onChange={(event) => setCreateFieldsFromHeader(event.target.checked)}
              />
              <span>{t("import.createFieldsFromHeader")}</span>
            </label>
          </>
        )}

        <div className="dialog-actions dialog-actions--start">
          <Button variant="secondary" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="ghost" onClick={() => setShowHelp(true)}>
            {t("import.help")}
          </Button>
          <Button onClick={() => void handlePreview()} disabled={loadingPreview}>
            {loadingPreview ? t("common.loading") : t("import.preview")}
          </Button>
        </div>

        {error ? <div className="inline-error">{error}</div> : null}

        {preview ? (
          <section className="import-preview">
            <div className="surface-muted">
              <div className="surface-muted__label">{t("import.beforeImport")}</div>
              <div className="detail-inline-stats detail-inline-stats--wrap">
                <span>
                  {t("import.target")}: {targetMode === "existing" ? selectedDeck?.name ?? "-" : newDeckName || "-"}
                </span>
                <span>
                  {t("import.detectedColumns")}: {preview.detected_columns.length}
                </span>
                <span>
                  {t("import.firstRowHeader")}: {hasHeader ? t("common.confirm") : t("common.cancel")}
                </span>
                <span>
                  {t("import.requiredMappedFields")}: {preview.unmapped_required_fields.length === 0 ? t("common.confirm") : preview.unmapped_required_fields.join(", ")}
                </span>
              </div>
            </div>

            <div className="stat-grid stat-grid--compact">
              <div className="stat-chip">
                <span>{t("import.parsed")}</span>
                <strong>{preview.summary.total_parsed}</strong>
              </div>
              <div className="stat-chip">
                <span>{t("import.importable")}</span>
                <strong>{preview.summary.importable}</strong>
              </div>
              <div className="stat-chip">
                <span>{t("import.duplicates")}</span>
                <strong>{preview.summary.duplicates}</strong>
              </div>
              <div className="stat-chip">
                <span>{t("import.missingRequired")}</span>
                <strong>{preview.summary.missing_required}</strong>
              </div>
            </div>

            <div className="surface-muted">
              <div className="surface-muted__label">{t("import.mappingSection")}</div>
              <p>{t("import.mappingGuide")}</p>
              <div className="form-stack">
                {preview.detected_columns.map((column, index) => (
                  <div key={column.column_index} className="schema-field-row">
                    <div className="field field--grow">
                      <span>{column.label}</span>
                      <div className="field-readonly">{t("import.mappingColumn", { number: column.column_index + 1 })}</div>
                    </div>

                    {targetMode === "existing" ? (
                      <label className="field field--grow">
                        <span>{t("import.deckField")}</span>
                        <select
                          value={mappings.find((mapping) => mapping.column_index === column.column_index)?.field_id ?? ""}
                          onChange={(event) => updateExistingFieldMapping(column.column_index, event.target.value ? Number(event.target.value) : null)}
                        >
                          <option value="">{t("common.ignore")}</option>
                          {selectedDeckFields.map((field) => (
                            <option
                              key={field.id}
                              value={field.id}
                              disabled={isExistingFieldMappingDisabled(mappings, column.column_index, field.id)}
                            >
                              {field.label}
                            </option>
                          ))}
                        </select>
                      </label>
                    ) : (
                      <>
                        <label className="field field--grow">
                          <span>{t("import.fieldLabel")}</span>
                          <input
                            dir="auto"
                            value={mappings.find((mapping) => mapping.column_index === column.column_index)?.label ?? ""}
                            onChange={(event) => updateNewFieldMapping(column.column_index, { label: event.target.value })}
                          />
                        </label>
                        <label className="field">
                          <span>{t("import.preset")}</span>
                          <select
                            value={mappings.find((mapping) => mapping.column_index === column.column_index)?.language_code ?? ""}
                            onChange={(event) => updateNewFieldMapping(column.column_index, { language_code: event.target.value || null })}
                          >
                            <option value="">{t("common.custom")}</option>
                            {settings.field_presets.map((preset) => (
                              <option key={preset.id} value={preset.id}>
                                {preset.label}
                              </option>
                            ))}
                          </select>
                        </label>
                        <label className="field field--checkbox">
                          <input
                            type="checkbox"
                            checked={Boolean(mappings.find((mapping) => mapping.column_index === column.column_index)?.required)}
                            onChange={(event) => updateNewFieldMapping(column.column_index, { required: event.target.checked })}
                          />
                          <span>{t("import.requiredField")}</span>
                        </label>
                      </>
                    )}
                  </div>
                ))}
              </div>
            </div>

            {preview.unmapped_required_fields.length > 0 ? (
              <div className="surface-muted">
                <div className="surface-muted__label">{t("import.unmappedRequired")}</div>
                <div className="detail-inline-stats detail-inline-stats--wrap">
                  {preview.unmapped_required_fields.map((field) => (
                    <span key={field}>{field}</span>
                  ))}
                </div>
              </div>
            ) : null}

            <div className="surface-muted">
              <div className="surface-muted__label">{t("import.previewSection")}</div>
              <p>{t("import.previewReady")}</p>
            </div>

            <div className="table-shell">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>{t("import.line")}</th>
                    {preview.detected_columns.map((column) => (
                      <th key={column.column_index}>{column.label}</th>
                    ))}
                    <th>{t("common.status")}</th>
                  </tr>
                </thead>
                <tbody>
                  {preview.rows.slice(0, 12).map((row) => (
                    <tr key={row.line_number}>
                      <td>{row.line_number}</td>
                      {row.columns.map((column, index) => (
                        <td key={`${row.line_number}-${index}`}>
                          <FieldText value={column} />
                        </td>
                      ))}
                      <td>
                        {row.duplicate ? (
                          <div className="status-stack">
                            <span className="pill pill--danger">{t("import.statusDuplicate")}</span>
                            <span className="table-note">{row.duplicate_reason}</span>
                          </div>
                        ) : row.missing_required_fields.length > 0 ? (
                          <div className="status-stack">
                            <span className="pill pill--danger">{t("import.statusMissingRequired")}</span>
                            <span className="table-note">{row.missing_required_fields.join(", ")}</span>
                          </div>
                        ) : (
                          <span className="pill">{t("import.statusImport")}</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {preview.invalid_lines.length > 0 ? (
              <div className="surface-muted">
                <div className="surface-muted__label">{t("import.invalidLines")}</div>
                <ul className="simple-list">
                  {preview.invalid_lines.slice(0, 6).map((line) => (
                    <li key={line.line_number}>
                      {t("import.line")} {line.line_number}: {line.reason}
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}

            <div className="dialog-actions">
              <Button variant="secondary" onClick={() => void handlePreview()}>
                {t("import.preview")}
              </Button>
              <Button onClick={() => void handleCommit()} disabled={!preview.ready_for_commit || preview.summary.importable === 0 || committing}>
                {committing ? t("common.loading") : t("import.commit")}
              </Button>
            </div>
          </section>
        ) : null}
      </div>
      <ImportHelpModal open={showHelp} onClose={() => setShowHelp(false)} />
    </Modal>
  );
}
