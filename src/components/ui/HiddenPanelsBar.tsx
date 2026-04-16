import { Button } from "./Button";
import { useI18n } from "../../lib/i18n";

interface HiddenPanelsBarProps {
  panels: Array<{ id: string; label: string }>;
  onShow: (panelId: string) => void;
}

export function HiddenPanelsBar({ panels, onShow }: HiddenPanelsBarProps) {
  const { t } = useI18n();

  if (panels.length === 0) {
    return null;
  }

  return (
    <section className="surface-muted hidden-panels-bar">
      <div className="surface-muted__label">{t("panel.hidden")}</div>
      <div className="dialog-actions dialog-actions--start">
        {panels.map((panel) => (
          <Button key={panel.id} type="button" variant="secondary" onClick={() => onShow(panel.id)}>
            {t("panel.show", { panel: panel.label })}
          </Button>
        ))}
      </div>
    </section>
  );
}
