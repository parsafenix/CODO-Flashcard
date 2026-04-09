import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import { FieldText } from "../../components/ui/FieldText";
import { api } from "../../lib/api";
import type { CommitImportRequest, DeckSummary, ImportPreviewRequest, ImportPreviewResponse } from "../../lib/types";
import { ImportHelpModal } from "./ImportHelpModal";

interface ImportWizardProps {
  open: boolean;
  decks: DeckSummary[];
  fixedDeck?: DeckSummary | null;
  defaultDelimiter: string;
  onClose: () => void;
  onImported: (deckId: number) => Promise<void> | void;
}

export function ImportWizard({ open: isOpen, decks, fixedDeck, defaultDelimiter, onClose, onImported }: ImportWizardProps) {
  const [filePath, setFilePath] = useState("");
  const [delimiter, setDelimiter] = useState(defaultDelimiter || "|");
  const [hasHeader, setHasHeader] = useState(false);
  const [targetMode, setTargetMode] = useState<"existing" | "new">(fixedDeck ? "existing" : "new");
  const [existingDeckId, setExistingDeckId] = useState<number>(fixedDeck?.id ?? decks[0]?.id ?? 0);
  const [newDeckName, setNewDeckName] = useState("");
  const [newDeckDescription, setNewDeckDescription] = useState("");
  const [preview, setPreview] = useState<ImportPreviewResponse | null>(null);
  const [applyHeaderLabelsToExisting, setApplyHeaderLabelsToExisting] = useState(false);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [committing, setCommitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showHelp, setShowHelp] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setDelimiter(defaultDelimiter || "|");
    setTargetMode(fixedDeck ? "existing" : "new");
    setExistingDeckId(fixedDeck?.id ?? decks[0]?.id ?? 0);
    setPreview(null);
    setFilePath("");
    setNewDeckName("");
    setNewDeckDescription("");
    setHasHeader(false);
    setApplyHeaderLabelsToExisting(false);
    setError(null);
  }, [defaultDelimiter, decks, fixedDeck, isOpen]);

  async function chooseFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Text files", extensions: ["txt"] }]
    });

    if (typeof selected === "string") {
      setFilePath(selected);
      if (!newDeckName) {
        const filename = selected.split(/[\\/]/).pop()?.replace(/\.txt$/i, "") ?? "";
        setNewDeckName(filename);
      }
    }
  }

  function buildPreviewRequest(): ImportPreviewRequest {
    return {
      file_path: filePath,
      delimiter,
      has_header: hasHeader,
      target:
        targetMode === "existing"
          ? { mode: "existing", deck_id: fixedDeck?.id ?? existingDeckId }
          : {
              mode: "new",
              name: newDeckName,
              description: newDeckDescription
            }
    };
  }

  async function handlePreview() {
    if (!filePath) {
      setError("Choose a UTF-8 text file first.");
      return;
    }

    if (!delimiter.trim()) {
      setError("Enter an import delimiter before previewing.");
      return;
    }

    if (targetMode === "new" && !newDeckName.trim()) {
      setError("Enter a name for the new deck before previewing.");
      return;
    }

    setLoadingPreview(true);
    setError(null);
    try {
      const response = await api.previewImport(buildPreviewRequest());
      setPreview(response);
      setApplyHeaderLabelsToExisting(response.can_update_existing_labels);
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Preview failed.";
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
      const request: CommitImportRequest = {
        ...buildPreviewRequest(),
        apply_header_labels_to_existing: applyHeaderLabelsToExisting
      };
      const result = await api.commitImport(request);
      await onImported(result.deck_id);
      onClose();
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Import failed.";
      setError(message);
    } finally {
      setCommitting(false);
    }
  }

  return (
    <Modal
      open={isOpen}
      title="Import vocabulary"
      description="Import UTF-8 text files with one card per line: Persian | English | Italian."
      onClose={onClose}
      width="large"
    >
      <div className="form-stack">
        <div className="field-row">
          <label className="field field--grow">
            <span>Text file</span>
            <input value={filePath} readOnly placeholder="Choose a local .txt file" />
          </label>
          <Button variant="secondary" onClick={() => void chooseFile()}>
            Choose file
          </Button>
        </div>

        <div className="field-grid field-grid--triple">
          <label className="field">
            <span>Delimiter</span>
            <input value={delimiter} onChange={(event) => setDelimiter(event.target.value)} maxLength={3} />
          </label>
          <label className="field field--checkbox">
            <input type="checkbox" checked={hasHeader} onChange={(event) => setHasHeader(event.target.checked)} />
            <span>First row is header</span>
          </label>
          {!fixedDeck ? (
            <label className="field">
              <span>Import target</span>
              <select value={targetMode} onChange={(event) => setTargetMode(event.target.value as "existing" | "new")}>
                <option value="new">Create new deck</option>
                <option value="existing">Use existing deck</option>
              </select>
            </label>
          ) : (
            <div className="field">
              <span>Target deck</span>
              <div className="field-readonly">{fixedDeck.name}</div>
            </div>
          )}
        </div>

        {targetMode === "existing" ? (
          <label className="field">
            <span>Existing deck</span>
            <select value={fixedDeck?.id ?? existingDeckId} onChange={(event) => setExistingDeckId(Number(event.target.value))}>
              {decks.map((deck) => (
                <option key={deck.id} value={deck.id}>
                  {deck.name}
                </option>
              ))}
            </select>
          </label>
        ) : (
          <div className="field-grid field-grid--dual">
            <label className="field">
              <span>New deck name</span>
              <input dir="auto" value={newDeckName} onChange={(event) => setNewDeckName(event.target.value)} />
            </label>
            <label className="field">
              <span>Description</span>
              <input dir="auto" value={newDeckDescription} onChange={(event) => setNewDeckDescription(event.target.value)} />
            </label>
          </div>
        )}

        <div className="dialog-actions dialog-actions--start">
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="ghost" onClick={() => setShowHelp(true)}>
            Help
          </Button>
          <Button onClick={() => void handlePreview()} disabled={loadingPreview}>
            {loadingPreview ? "Previewing..." : "Preview import"}
          </Button>
        </div>

        {error ? <div className="inline-error">{error}</div> : null}

        {preview ? (
          <section className="import-preview">
            <div className="stat-grid stat-grid--compact">
              <div className="stat-chip">
                <span>Parsed</span>
                <strong>{preview.summary.total_parsed}</strong>
              </div>
              <div className="stat-chip">
                <span>Importable</span>
                <strong>{preview.summary.importable}</strong>
              </div>
              <div className="stat-chip">
                <span>Duplicates</span>
                <strong>{preview.summary.duplicates}</strong>
              </div>
              <div className="stat-chip">
                <span>Invalid</span>
                <strong>{preview.summary.invalid}</strong>
              </div>
            </div>

            {preview.header_labels ? (
              <div className="surface-muted">
                <div className="surface-muted__label">Detected header</div>
                <div className="field-grid field-grid--triple">
                  {preview.header_labels.map((label, index) => (
                    <FieldText key={index} value={label} />
                  ))}
                </div>
                {preview.can_update_existing_labels ? (
                  <label className="field field--checkbox">
                    <input
                      type="checkbox"
                      checked={applyHeaderLabelsToExisting}
                      onChange={(event) => setApplyHeaderLabelsToExisting(event.target.checked)}
                    />
                    <span>Update this deck's language labels from the header during import</span>
                  </label>
                ) : null}
              </div>
            ) : null}

            <div className="table-shell">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Line</th>
                    <th>Language 1</th>
                    <th>Language 2</th>
                    <th>Language 3</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {preview.rows.slice(0, 12).map((row) => (
                    <tr key={row.line_number}>
                      <td>{row.line_number}</td>
                      <td><FieldText value={row.language_1} /></td>
                      <td><FieldText value={row.language_2} /></td>
                      <td><FieldText value={row.language_3} /></td>
                      <td>
                        {row.duplicate ? (
                          <div className="status-stack">
                            <span className="pill pill--danger">Duplicate</span>
                            <span className="table-note">{row.duplicate_reason}</span>
                          </div>
                        ) : (
                          <span className="pill">Import</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {preview.invalid_lines.length > 0 ? (
              <div className="surface-muted">
                <div className="surface-muted__label">Invalid lines</div>
                <ul className="simple-list">
                  {preview.invalid_lines.slice(0, 6).map((line) => (
                    <li key={line.line_number}>
                      Line {line.line_number}: {line.reason}
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}

            <div className="dialog-actions">
              <Button variant="secondary" onClick={() => setPreview(null)}>
                Adjust import
              </Button>
              <Button onClick={() => void handleCommit()} disabled={preview.summary.importable === 0 || committing}>
                {committing ? "Importing..." : "Import cards"}
              </Button>
            </div>
          </section>
        ) : null}
      </div>
      <ImportHelpModal open={showHelp} onClose={() => setShowHelp(false)} />
    </Modal>
  );
}
