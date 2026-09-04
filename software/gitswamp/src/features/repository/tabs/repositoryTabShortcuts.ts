export interface RepositoryTabShortcutHandlers {
  newTab: () => void;
  closeActiveTab: () => void;
  reopenClosedTab: () => void;
  selectAdjacentTab: (direction: 1 | -1) => void;
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

  if (key === "tab") {
    event.preventDefault();
    handlers.selectAdjacentTab(event.shiftKey ? -1 : 1);
    return true;
  }

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
