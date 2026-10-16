export const BASES: number[] = [2, 8, 10, 16, 36];

export function convertNumber(input: string, fromBase: number, toBase: number): { value: string; error: string | null } {
  if (!input.trim()) return { value: "", error: null };
  const i = input.trim();
  try {
    const num = parseInt(i, fromBase);
    if (Number.isNaN(num) || !Number.isFinite(num)) {
      return { value: "", error: "Invalid number for selected base." };
    }
    if (toBase === 10) return { value: num.toString(10), error: null };
    if (toBase === 16) return { value: num.toString(16).toUpperCase(), error: null };
    if (toBase === 8) return { value: num.toString(8), error: null };
    if (toBase === 2) return { value: num.toString(2), error: null };
    return { value: num.toString(toBase).toUpperCase(), error: null };
  } catch (err) {
    return { value: "", error: err instanceof Error ? err.message : String(err) };
  }
}
