import { type PropsWithChildren, useEffect } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "../../lib/i18n";

interface ModalProps {
  open: boolean;
  title: string;
  description?: string;
  onClose: () => void;
  width?: "medium" | "large";
}

export function Modal({ open, title, description, onClose, width = "medium", children }: PropsWithChildren<ModalProps>) {
  const { t } = useI18n();

  useEffect(() => {
    if (!open) {
      return;
    }

    const listener = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [onClose, open]);

  if (!open) {
    return null;
  }

  return createPortal(
    <div className="modal-overlay" onMouseDown={onClose}>
      <section
        className={`modal modal--${width}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="modal__header">
          <div>
            <h2 id="modal-title">{title}</h2>
            {description ? <p>{description}</p> : null}
          </div>
          <button className="icon-button" aria-label={t("common.close")} onClick={onClose}>
            <span aria-hidden="true">×</span>
          </button>
        </header>
        <div className="modal__body">{children}</div>
      </section>
    </div>,
    document.body
  );
}
