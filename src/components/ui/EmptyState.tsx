import type { PropsWithChildren, ReactNode } from "react";
import { useI18n } from "../../lib/i18n";

interface EmptyStateProps {
  title: string;
  description: string;
  actions?: ReactNode;
}

export function EmptyState({ title, description, actions, children }: PropsWithChildren<EmptyStateProps>) {
  const { t } = useI18n();

  return (
    <div className="empty-state">
      <div className="empty-state__eyebrow">{t("empty.nothingYet")}</div>
      <h2>{title}</h2>
      <p>{description}</p>
      {actions ? <div className="empty-state__actions">{actions}</div> : null}
      {children}
    </div>
  );
}
