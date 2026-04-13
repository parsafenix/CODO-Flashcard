import { type FormEvent, useEffect, useMemo, useState } from "react";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { Modal } from "../../components/ui/Modal";
import { useI18n } from "../../lib/i18n";
import type { NormalizedApiError } from "../../lib/api";
import { getActiveFields } from "../../lib/deckFields";
import type { CreateDeckInput, DeckFieldInput, DeckSummary, UpdateDeckInput } from "../../lib/types";
import { summarizeDeckFields } from "./schemaSummary";

interface DeckFormModalProps {
  open: boolean;
  initialDeck?: DeckSummary | null;
  onClose: () => void;
  onSubmit: (input: CreateDeckInput | UpdateDeckInput) => Promise<void>;
}

function buildDefaultFields(presetIds: string[]): DeckFieldInput[] {
  const defaults = presetIds.length >= 2 ? presetIds.slice(0, 2) : ["field-1", "field-2"];
  return defaults.map((presetId, index) => ({
    label: presetId
      .split("-")
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join(" "),
    language_code: presetId.startsWith("field-") ? null : presetId,
    order_index: index,
    required: true,
    active: true,
    field_type: "text",
  }));
}

export function DeckFormModal({ open, initialDeck, onClose, onSubmit }: DeckFormModalProps) {
  const { settings } = useAppContext();
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [fields, setFields] = useState<DeckFieldInput[]>([]);
  const [deletedFieldIds, setDeletedFieldIds] = useState<number[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldError, setFieldError] = useState<string | null>(null);
  const [pendingDeleteField, setPendingDeleteField] = useState<DeckFieldInput | null>(null);

  const presetOptions = useMemo(() => settings.field_presets, [settings.field_presets]);

  useEffect(() => {
    if (!open) {
      return;
    }

    setName(initialDeck?.name ?? "");
    setDescription(initialDeck?.description ?? "");
    setFields(
      initialDeck?.fields.length
        ? initialDeck.fields.map((field) => ({
            id: field.id,
            label: field.label,
            language_code: field.language_code,
            order_index: field.order_index,
            required: field.required,
            active: field.active,
            field_type: field.field_type,
          }))
        : buildDefaultFields(
            settings.field_presets
              .filter((preset) => preset.kind === "language")
              .map((preset) => preset.id)
          )
    );
    setDeletedFieldIds([]);
    setError(null);
    setFieldError(null);
  }, [initialDeck, open, settings.field_presets]);

  function updateField(index: number, patch: Partial<DeckFieldInput>) {
    setFields((current) =>
      current.map((field, fieldIndex) =>
        fieldIndex === index
          ? {
              ...field,
              ...patch,
              order_index: fieldIndex,
            }
          : field
      )
    );
  }

  function moveField(index: number, direction: -1 | 1) {
    const nextIndex = index + direction;
    if (nextIndex < 0 || nextIndex >= fields.length) {
      return;
    }

    setFields((current) => {
      const copy = [...current];
      const [item] = copy.splice(index, 1);
      copy.splice(nextIndex, 0, item);
      return copy.map((field, fieldIndex) => ({ ...field, order_index: fieldIndex }));
    });
  }

  function addField() {
    setFields((current) => [
      ...current,
      {
        label: `Field ${current.length + 1}`,
        language_code: null,
        order_index: current.length,
        required: false,
        active: true,
        field_type: "text",
      },
    ]);
  }

  function requestRemoveField(index: number) {
    const field = fields[index];
    if (!field?.id) {
      setFields((current) => current.filter((_, itemIndex) => itemIndex !== index).map((item, itemIndex) => ({ ...item, order_index: itemIndex })));
      return;
    }
    setPendingDeleteField(field);
  }

  function confirmRemoveField() {
    if (!pendingDeleteField?.id) {
      setPendingDeleteField(null);
      return;
    }
    setDeletedFieldIds((current) => [...current, pendingDeleteField.id!]);
    setFields((current) =>
      current
        .filter((field) => field.id !== pendingDeleteField.id)
        .map((field, index) => ({
          ...field,
          order_index: index,
        }))
    );
    setPendingDeleteField(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    setFieldError(null);

    try {
      const payload = {
        name,
        description,
        fields: fields.map((field, index) => ({ ...field, order_index: index })),
      };

      await onSubmit(
        initialDeck
          ? {
              id: initialDeck.id,
              deleted_field_ids: deletedFieldIds,
              ...payload,
            }
          : payload
      );
      onClose();
    } catch (err) {
      const apiError = err as NormalizedApiError;
      const message = typeof apiError?.message === "string" ? apiError.message : t("deckform.error");
      if (apiError?.field === "name") {
        setFieldError(message);
      } else {
        setError(message);
      }
    } finally {
      setSubmitting(false);
    }
  }

  const activeFields = getActiveFields(
    fields.map((field, index) => ({
      id: field.id ?? -1 - index,
      deck_id: initialDeck?.id ?? 0,
      label: field.label,
      language_code: field.language_code ?? null,
      order_index: index,
      required: field.required,
      active: field.active,
      field_type: field.field_type ?? "text",
      system_key: null,
    }))
  );
  const schemaSummary = summarizeDeckFields(fields);

  return (
    <>
      <Modal
        open={open}
        title={initialDeck ? t("deckform.editTitle") : t("deckform.createTitle")}
        description={t("deckform.description")}
        onClose={onClose}
        width="large"
      >
        <form className="form-stack" onSubmit={handleSubmit}>
          <label className="field">
            <span>{t("deckform.deckName")}</span>
            <input dir="auto" value={name} onChange={(event) => setName(event.target.value)} />
            {fieldError ? <div className="field-error">{fieldError}</div> : null}
          </label>

          <label className="field">
            <span>{t("deckform.deckDescription")}</span>
            <textarea rows={3} dir="auto" value={description} onChange={(event) => setDescription(event.target.value)} />
          </label>

          <div className="surface-muted">
            <div className="surface-muted__label">{t("deckform.fields")}</div>
            <div className="schema-explainer">
              <div className="surface-muted">
                <div className="surface-muted__label">{t("deckform.explainerTitle")}</div>
                <ul className="simple-list">
                  <li>{t("deckform.explainerRequired")}</li>
                  <li>{t("deckform.explainerActive")}</li>
                  <li>{t("deckform.explainerPresets")}</li>
                </ul>
              </div>
              <div className="surface-muted">
                <div className="surface-muted__label">{t("deckform.summaryTitle")}</div>
                <div className="detail-inline-stats detail-inline-stats--wrap">
                  <span>
                    {t("deckform.summaryActive")}: {schemaSummary.activeCount}
                  </span>
                  <span>
                    {t("deckform.summaryRequired")}: {schemaSummary.requiredCount}
                  </span>
                  <span>
                    {t("deckform.summaryPrompt")}: {schemaSummary.promptEligibleCount}
                  </span>
                  <span>
                    {t("deckform.summaryReveal")}: {schemaSummary.revealEligibleCount}
                  </span>
                </div>
              </div>
            </div>
            <div className="surface-muted">
              <div className="surface-muted__label">{t("deckform.fieldActionsTitle")}</div>
              <p>{t("deckform.fieldActionsSafe")}</p>
              <p>{t("deckform.fieldActionsDelete")}</p>
            </div>
            <div className="form-stack">
              {fields.map((field, index) => (
                <div key={field.id ?? `new-${index}`} className="schema-field-row">
                  <label className="field field--grow">
                    <span>{t("deckform.fieldLabel")}</span>
                    <input dir="auto" value={field.label} onChange={(event) => updateField(index, { label: event.target.value })} />
                  </label>

                  <label className="field">
                    <span>{t("deckform.fieldPreset")}</span>
                    <select
                      value={field.language_code ?? ""}
                      onChange={(event) => {
                        const presetId = event.target.value || null;
                        const preset = presetOptions.find((item) => item.id === presetId);
                        updateField(index, {
                          language_code: presetId,
                          label: field.label.trim() ? field.label : preset?.label ?? field.label,
                        });
                      }}
                    >
                      <option value="">{t("common.custom")}</option>
                      {presetOptions.map((preset) => (
                        <option key={preset.id} value={preset.id}>
                          {preset.label}
                        </option>
                      ))}
                    </select>
                  </label>

                  <label className="field field--checkbox">
                    <input
                      type="checkbox"
                      checked={field.required}
                      onChange={(event) => updateField(index, { required: event.target.checked })}
                    />
                    <span>{t("deckform.required")}</span>
                  </label>

                  <label className="field field--checkbox">
                    <input
                      type="checkbox"
                      checked={field.active}
                      onChange={(event) => updateField(index, { active: event.target.checked })}
                    />
                    <span>{t("deckform.active")}</span>
                  </label>

                  <div className="schema-field-row__actions">
                    <div className="inline-actions">
                      <Button variant="ghost" type="button" onClick={() => moveField(index, -1)} disabled={index === 0}>
                        {t("deckform.moveUp")}
                      </Button>
                      <Button variant="ghost" type="button" onClick={() => moveField(index, 1)} disabled={index === fields.length - 1}>
                        {t("deckform.moveDown")}
                      </Button>
                    </div>
                    <Button variant="danger" type="button" onClick={() => requestRemoveField(index)}>
                      {t("deckform.removeField")}
                    </Button>
                  </div>
                </div>
              ))}
            </div>

            <div className="dialog-actions dialog-actions--start">
              <Button type="button" variant="secondary" onClick={addField}>
                {t("deckform.addField")}
              </Button>
            </div>
          </div>

          <div className="surface-muted">
            <div className="surface-muted__label">{t("deckform.preview")}</div>
            <div className="detail-inline-stats">
              {activeFields.map((field) => (
                <span key={field.id}>{field.label}</span>
              ))}
            </div>
          </div>

          {error ? <div className="inline-error">{error}</div> : null}

          <div className="dialog-actions">
            <Button type="button" variant="secondary" onClick={onClose}>
              {t("common.cancel")}
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? t("common.loading") : initialDeck ? t("common.save") : t("common.create")}
            </Button>
          </div>
        </form>
      </Modal>

      <ConfirmDialog
        open={Boolean(pendingDeleteField)}
        title={t("deckform.deleteFieldTitle")}
        description={t("deckform.deleteFieldDescription", { label: pendingDeleteField?.label ?? "" })}
        confirmLabel={t("common.delete")}
        onCancel={() => setPendingDeleteField(null)}
        onConfirm={confirmRemoveField}
      />
    </>
  );
}
