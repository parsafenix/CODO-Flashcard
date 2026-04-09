import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";

const exampleText = `Persian | English | Italian
سلام | Hello | Ciao
کتاب | Book | Libro`;

interface ImportHelpModalProps {
  open: boolean;
  onClose: () => void;
}

export function ImportHelpModal({ open, onClose }: ImportHelpModalProps) {
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
    <Modal
      open={open}
      onClose={onClose}
      title="Import guide"
      description="Use a UTF-8 text file with one flashcard per line and exactly three required columns."
      width="large"
    >
      <div className="import-help">
        <div className="surface-muted">
          <div className="surface-muted__label">How it works</div>
          <ul className="simple-list">
            <li>Each line creates one card.</li>
            <li>Use the pipe character `|` to separate values.</li>
            <li>The expected v1 format is `Persian | English | Italian`.</li>
            <li>The first line can optionally be a header.</li>
            <li>Empty lines are ignored.</li>
            <li>Lines starting with `#` are treated as comments and ignored.</li>
            <li>Duplicate rows are skipped because strict duplicate protection is enabled.</li>
            <li>The file should be saved as UTF-8 text.</li>
            <li>Exactly 3 required columns are expected in this version.</li>
          </ul>
        </div>

        <div className="surface-muted">
          <div className="surface-muted__label">Example</div>
          <pre className="import-help__code" dir="auto">
            {exampleText}
          </pre>
          <div className="dialog-actions dialog-actions--start">
            <Button variant="secondary" onClick={() => void copyExample()}>
              {copied ? "Copied" : "Copy example"}
            </Button>
          </div>
        </div>
      </div>
    </Modal>
  );
}
