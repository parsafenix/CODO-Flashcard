import { Button } from "./Button";
import { Modal } from "./Modal";
import { useI18n } from "../../lib/i18n";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  confirmLabel?: string;
  confirmVariant?: "primary" | "danger";
  onCancel: () => void;
  onConfirm: () => void;
}

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel,
  confirmVariant = "danger",
  onCancel,
  onConfirm
}: ConfirmDialogProps) {
  const { t } = useI18n();

  return (
    <Modal open={open} title={title} description={description} onClose={onCancel}>
      <div className="dialog-actions">
        <Button variant="secondary" onClick={onCancel}>
          {t("common.cancel")}
        </Button>
        <Button variant={confirmVariant} onClick={onConfirm}>
          {confirmLabel ?? t("common.confirm")}
        </Button>
      </div>
    </Modal>
  );
}
