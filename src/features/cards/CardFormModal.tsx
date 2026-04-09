import { type FormEvent, useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import type { CardRecord, DeckSummary } from "../../lib/types";
import type { NormalizedApiError } from "../../lib/api";

interface CardFormModalProps {
  open: boolean;
  deck: DeckSummary;
  initialCard?: CardRecord | null;
  onClose: () => void;
  onSubmit: (input: {
    id?: number;
    deck_id: number;
    language_1: string;
    language_2: string;
    language_3: string;
    note?: string;
    example_sentence?: string;
    tag?: string;
  }) => Promise<void>;
}

export function CardFormModal({ open, deck, initialCard, onClose, onSubmit }: CardFormModalProps) {
  const [language1, setLanguage1] = useState("");
  const [language2, setLanguage2] = useState("");
  const [language3, setLanguage3] = useState("");
  const [note, setNote] = useState("");
  const [example, setExample] = useState("");
  const [tag, setTag] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [fieldError, setFieldError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!open) {
      return;
    }

    setLanguage1(initialCard?.language_1 ?? "");
    setLanguage2(initialCard?.language_2 ?? "");
    setLanguage3(initialCard?.language_3 ?? "");
    setNote(initialCard?.note ?? "");
    setExample(initialCard?.example_sentence ?? "");
    setTag(initialCard?.tag ?? "");
    setError(null);
    setFieldError(null);
  }, [initialCard, open]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    setFieldError(null);

    try {
      await onSubmit({
        id: initialCard?.id,
        deck_id: deck.id,
        language_1: language1,
        language_2: language2,
        language_3: language3,
        note,
        example_sentence: example,
        tag
      });
      onClose();
    } catch (err) {
      const apiError = err as NormalizedApiError;
      const message = typeof apiError?.message === "string" ? apiError.message : "Unable to save card.";
      if (
        apiError?.field === "language_1" ||
        apiError?.field === "language_2" ||
        apiError?.field === "language_3"
      ) {
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
      title={initialCard ? "Edit card" : "Add card"}
      description="Store the original vocabulary exactly as entered. Duplicate checks use a normalized copy behind the scenes."
      onClose={onClose}
      width="large"
    >
      <form className="form-stack" onSubmit={handleSubmit}>
        <div className="field-grid field-grid--triple">
          <label className="field">
            <span>{deck.language_1_label}</span>
            <input dir="auto" value={language1} onChange={(event) => setLanguage1(event.target.value)} />
            {fieldError ? <div className="field-error">{fieldError}</div> : null}
          </label>
          <label className="field">
            <span>{deck.language_2_label}</span>
            <input dir="auto" value={language2} onChange={(event) => setLanguage2(event.target.value)} />
          </label>
          <label className="field">
            <span>{deck.language_3_label}</span>
            <input dir="auto" value={language3} onChange={(event) => setLanguage3(event.target.value)} />
          </label>
        </div>

        <label className="field">
          <span>Note</span>
          <textarea rows={2} dir="auto" value={note} onChange={(event) => setNote(event.target.value)} />
        </label>

        <label className="field">
          <span>Example sentence</span>
          <textarea rows={3} dir="auto" value={example} onChange={(event) => setExample(event.target.value)} />
        </label>

        <label className="field">
          <span>Tag or category</span>
          <input dir="auto" value={tag} onChange={(event) => setTag(event.target.value)} placeholder="Travel, verbs, daily life..." />
        </label>

        {error ? <div className="inline-error">{error}</div> : null}

        <div className="dialog-actions">
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={submitting}>
            {submitting ? "Saving..." : initialCard ? "Save changes" : "Add card"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
