export function formatRelativeDate(value: string | null): string {
  if (!value) {
    return "Never";
  }

  const date = new Date(value);
  const delta = date.getTime() - Date.now();
  const abs = Math.abs(delta);
  const minutes = Math.round(abs / 60000);

  if (minutes < 1) {
    return "Just now";
  }

  if (minutes < 60) {
    return delta < 0 ? `${minutes}m ago` : `in ${minutes}m`;
  }

  const hours = Math.round(minutes / 60);
  if (hours < 24) {
    return delta < 0 ? `${hours}h ago` : `in ${hours}h`;
  }

  const days = Math.round(hours / 24);
  return delta < 0 ? `${days}d ago` : `in ${days}d`;
}

export function formatAccuracy(correct: number, studied: number): string {
  if (studied === 0) {
    return "0%";
  }

  return `${Math.round((correct / studied) * 100)}%`;
}

export function formatUtcDateLabel(value: string): string {
  const date = new Date(`${value}T00:00:00Z`);
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric"
  }).format(date);
}
