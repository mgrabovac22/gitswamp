export function isEditableTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  if (!element) return false;

  const tag = element.tagName.toLowerCase();
  return element.isContentEditable
    || tag === "input"
    || tag === "textarea"
    || tag === "select"
    || tag === "option";
}
