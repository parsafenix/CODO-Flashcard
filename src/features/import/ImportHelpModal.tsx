import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import { useI18n } from "../../lib/i18n";

const exampleText = `Persian | English
سلام | Hello

Persian | English | Italian
کتاب | Book | Libro

English | Definition | Persian | Example
book | A written work | کتاب | I bought a new book`;

interface ImportHelpModalProps {
  open: boolean;
  onClose: () => void;
}

export function ImportHelpModal({ open, onClose }: ImportHelpModalProps) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  async function copyExample() {
    try {
      await navigator.clipboard.writeText(exampleText);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  }

  return (
    <Modal open={open} onClose={onClose} title={t("import.guideTitle")} description={t("import.guideDescription")} width="large">
      <div className="import-help">
        <div className="surface-muted">
          <div className="surface-muted__label">{t("import.helpHowItWorks")}</div>
          <p>{t("import.helpIntro")}</p>
          <ul className="simple-list">
            <li>{t("import.helpRule.oneLine")}</li>
            <li>{t("import.helpRule.separator")}</li>
            <li>{t("import.helpRule.utf8")}</li>
            <li>{t("import.helpRule.columns")}</li>
            <li>{t("import.helpRule.header")}</li>
            <li>{t("import.helpRule.required")}</li>
            <li>{t("import.helpRule.duplicates")}</li>
            <li>{t("import.helpRule.comments")}</li>
          </ul>
        </div>

        <div className="surface-muted">
          <div className="surface-muted__label">{t("import.helpExamples")}</div>
          <div className="detail-inline-stats detail-inline-stats--wrap">
            <span>{t("import.helpExample2")}</span>
            <span>{t("import.helpExample3")}</span>
            <span>{t("import.helpExample4")}</span>
          </div>
          <pre className="import-help__code" dir="auto">
            {exampleText}
          </pre>

          <div className="surface-muted import-help__note">
            <div className="surface-muted__label">{t("import.helpHeaderTitle")}</div>
            <p>{t("import.helpHeaderDescription")}</p>
          </div>

          <div className="surface-muted import-help__note">
            <div className="surface-muted__label">{t("import.helpRequiredTitle")}</div>
            <p>{t("import.helpRequiredDescription")}</p>
          </div>

          <div className="dialog-actions dialog-actions--start">
            <Button variant="secondary" onClick={() => void copyExample()}>
              {copied ? t("common.copied") : t("common.copyExample")}
            </Button>
          </div>
        </div>
      </div>
    </Modal>
  );
}
