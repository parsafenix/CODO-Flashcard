import { type FormEvent, useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import type { CreateDeckInput, DeckSummary, UpdateDeckInput } from "../../lib/types";
import type { NormalizedApiError } from "../../lib/api";

interface DeckFormModalProps {
  open: boolean;
  initialDeck?: DeckSummary | null;
  onClose: () => void;
  onSubmit: (input: CreateDeckInput | UpdateDeckInput) => Promise<void>;
}

export function DeckFormModal({ open, initialDeck, onClose, onSubmit }: DeckFormModalProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [labels, setLabels] = useState(["Persian", "English", "Italian"]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldError, setFieldError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    setName(initialDeck?.name ?? "");
    setDescription(initialDeck?.description ?? "");
    setLabels([
      initialDeck?.language_1_label ?? "Persian",
      initialDeck?.language_2_label ?? "English",
      initialDeck?.language_3_label ?? "Italian"
    ]);
    setError(null);
    setFieldError(null);
  }, [initialDeck, open]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    setFieldError(null);

    try {
      const payload = {
        name,
        description,
        language_1_label: labels[0],
        language_2_label: labels[1],
        language_3_label: labels[2]
      };

      await onSubmit(
        initialDeck
          ? {
              id: initialDeck.id,
              ...payload
            }
          : payload
      );
      onClose();
    } catch (err) {
      const apiError = err as NormalizedApiError;
      const message = typeof apiError?.message === "string" ? apiError.message : "Unable to save deck.";
      if (apiError?.field === "name") {
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
      title={initialDeck ? "Edit deck" : "Create deck"}
      description="Set the deck metadata and the labels used across import, study, and export."
      onClose={onClose}
    >
      <form className="form-stack" onSubmit={handleSubmit}>
        <label className="field">
          <span>Deck name</span>
          <input dir="auto" value={name} onChange={(event) => setName(event.target.value)} placeholder="Everyday Persian" />
          {fieldError ? <div className="field-error">{fieldError}</div> : null}
        </label>

        <label className="field">
          <span>Description</span>
          <textarea
            rows={3}
            dir="auto"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            placeholder="Short description for this deck"
          />
        </label>

        <div className="field-grid field-grid--triple">
          {labels.map((label, index) => (
            <label key={index} className="field">
              <span>Language {index + 1} label</span>
              <input
                dir="auto"
                value={label}
                onChange={(event) =>
                  setLabels((current) => current.map((item, itemIndex) => (itemIndex === index ? event.target.value : item)))
                }
              />
            </label>
          ))}
        </div>

        {error ? <div className="inline-error">{error}</div> : null}

        <div className="dialog-actions">
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={submitting}>
            {submitting ? "Saving..." : initialDeck ? "Save deck" : "Create deck"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
