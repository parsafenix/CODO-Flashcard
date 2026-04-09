import type { PropsWithChildren, ReactNode } from "react";

interface EmptyStateProps {
  title: string;
  description: string;
  actions?: ReactNode;
}

export function EmptyState({ title, description, actions, children }: PropsWithChildren<EmptyStateProps>) {
  return (
    <div className="empty-state">
      <div className="empty-state__eyebrow">Nothing here yet</div>
      <h2>{title}</h2>
      <p>{description}</p>
      {actions ? <div className="empty-state__actions">{actions}</div> : null}
      {children}
    </div>
  );
}
