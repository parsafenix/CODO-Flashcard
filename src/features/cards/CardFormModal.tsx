import { type FormEvent, useEffect, useMemo, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import { getActiveFields, getCardValue } from "../../lib/deckFields";
import { useI18n } from "../../lib/i18n";
import type { NormalizedApiError } from "../../lib/api";
import type { CardRecord, DeckSummary } from "../../lib/types";

interface CardFormModalProps {
  open: boolean;
  deck: DeckSummary;
  initialCard?: CardRecord | null;
  onClose: () => void;
  onSubmit: (input: {
    id?: number;
    deck_id: number;
    values: Array<{ field_id: number; value: string }>;
  }) => Promise<void>;
}

export function CardFormModal({ open, deck, initialCard, onClose, onSubmit }: CardFormModalProps) {
  const { t } = useI18n();
  const activeFields = useMemo(() => getActiveFields(deck.fields), [deck.fields]);
  const [values, setValues] = useState<Record<number, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [fieldError, setFieldError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!open) {
      return;
    }

    setValues(
      Object.fromEntries(
        activeFields.map((field) => [field.id, initialCard ? getCardValue(initialCard.values, field.id) : ""])
      )
    );
    setError(null);
    setFieldError(null);
  }, [activeFields, initialCard, open]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    setFieldError(null);

    try {
      await onSubmit({
        id: initialCard?.id,
        deck_id: deck.id,
        values: activeFields.map((field) => ({
          field_id: field.id,
          value: values[field.id] ?? "",
        })),
      });
      onClose();
    } catch (err) {
      const apiError = err as NormalizedApiError;
      const message =
        apiError?.code === "duplicate_card"
          ? t("cardform.duplicate")
          : typeof apiError?.message === "string"
            ? apiError.message
            : t("cardform.error");
      if (apiError?.field === "values") {
        setFieldError(message);
      } else {
        setError(message);
      }
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Modal
      open={open}
      title={initialCard ? t("cardform.editTitle") : t("cardform.addTitle")}
      description={t("cardform.description")}
      onClose={onClose}
      width="large"
    >
      <form className="form-stack" onSubmit={handleSubmit}>
        <div className="field-grid field-grid--dynamic">
          {activeFields.map((field, index) => (
            <label key={field.id} className="field">
              <span>
                {field.label}
                {field.required ? " *" : ""}
              </span>
              <textarea
                rows={index < 2 ? 2 : 3}
                dir="auto"
                value={values[field.id] ?? ""}
                onChange={(event) =>
                  setValues((current) => ({
                    ...current,
                    [field.id]: event.target.value,
                  }))
                }
              />
            </label>
          ))}
        </div>

        {fieldError ? <div className="field-error">{fieldError}</div> : null}
        {error ? <div className="inline-error">{error}</div> : null}

        <div className="dialog-actions">
          <Button type="button" variant="secondary" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" disabled={submitting}>
            {submitting ? t("common.loading") : initialCard ? t("common.save") : t("common.add")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
