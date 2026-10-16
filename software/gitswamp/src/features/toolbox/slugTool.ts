export function slugify(input: string, lower = true): string {
  const trimmed = input.trim();
  if (!trimmed) return "";
  let value = trimmed
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^A-Za-z0-9\s_-]+/g, " ")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  if (lower) value = value.toLowerCase();
  return value;
}
