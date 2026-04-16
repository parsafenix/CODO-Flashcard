import type { PropsWithChildren, ReactNode } from "react";
import { Button } from "./Button";
import { useI18n } from "../../lib/i18n";

interface PanelCardProps {
  title: string;
  description?: string;
  actions?: ReactNode;
  onHide?: () => void;
  className?: string;
}

export function PanelCard({
  title,
  description,
  actions,
  onHide,
  className = "",
  children,
}: PropsWithChildren<PanelCardProps>) {
  const { t } = useI18n();

  return (
    <article className={`surface-panel dashboard-panel ${className}`.trim()}>
      <header className="dashboard-panel__header">
        <div className="dashboard-panel__title-group">
          <h2>{title}</h2>
          {description ? <p>{description}</p> : null}
        </div>
        <div className="dashboard-panel__actions">
          {actions}
          {onHide ? (
            <Button type="button" variant="ghost" onClick={onHide}>
              {t("panel.hide")}
            </Button>
          ) : null}
        </div>
      </header>
      <div className="dashboard-panel__body">{children}</div>
    </article>
  );
}
