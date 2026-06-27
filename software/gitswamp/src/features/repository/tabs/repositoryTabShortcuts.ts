export interface RepositoryTabShortcutHandlers {
  newTab: () => void;
  closeActiveTab: () => void;
  reopenClosedTab: () => void;
  canCloseActiveTab: () => boolean;
  canReopenClosedTab: () => boolean;
}

function isPrimaryTabShortcut(event: KeyboardEvent): boolean {
  return (event.ctrlKey || event.metaKey) && !event.altKey;
}

export function handleRepositoryTabShortcut(
  event: KeyboardEvent,
  handlers: RepositoryTabShortcutHandlers,
): boolean {
  if (!isPrimaryTabShortcut(event)) {
    return false;
  }

  const key = event.key.toLowerCase();

  if (!event.shiftKey && key === "t") {
    event.preventDefault();
    handlers.newTab();
    return true;
  }

  if (!event.shiftKey && key === "w") {
    event.preventDefault();
    if (handlers.canCloseActiveTab()) {
      handlers.closeActiveTab();
    }
    return true;
  }

  if (event.shiftKey && key === "t") {
    event.preventDefault();
    if (handlers.canReopenClosedTab()) {
      handlers.reopenClosedTab();
    }
    return true;
  }

  return false;
}
