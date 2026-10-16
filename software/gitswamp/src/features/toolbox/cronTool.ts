export const CRON_PRESETS: { label: string; expr: string }[] = [
  { label: "Every minute", expr: "* * * * *" },
  { label: "Every 5 minutes", expr: "*/5 * * * *" },
  { label: "Every 15 minutes", expr: "*/15 * * * *" },
  { label: "Every 30 minutes", expr: "*/30 * * * *" },
  { label: "Hourly", expr: "0 * * * *" },
  { label: "Every 6 hours", expr: "0 */6 * * *" },
  { label: "Daily at midnight", expr: "0 0 * * *" },
  { label: "Daily at 9am", expr: "0 9 * * *" },
  { label: "Weekdays at 9am", expr: "0 9 * * 1-5" },
  { label: "Weekly (Sunday)", expr: "0 0 * * 0" },
  { label: "Monthly (1st)", expr: "0 0 1 * *" },
  { label: "Yearly (Jan 1)", expr: "0 0 1 1 *" },
];

const ALIASES: Record<string, string> = {
  "@yearly": "0 0 1 1 *",
  "@annually": "0 0 1 1 *",
  "@monthly": "0 0 1 * *",
  "@weekly": "0 0 * * 0",
  "@daily": "0 0 * * *",
  "@midnight": "0 0 * * *",
  "@hourly": "0 * * * *",
};

const FIELD_RANGES = [
  { min: 0, max: 59, label: "minute" },
  { min: 0, max: 23, label: "hour" },
  { min: 1, max: 31, label: "day of month" },
  { min: 1, max: 12, label: "month" },
  { min: 0, max: 7, label: "day of week" },
];

const MONTH_NAMES = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DOW_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

export function normalizeCron(expr: string): string {
  const trimmed = expr.trim();
  return ALIASES[trimmed.toLowerCase()] ?? trimmed;
}

interface ExpandResult {
  set: Set<number> | null;
  error: string | null;
}

function expandField(part: string, min: number, max: number): ExpandResult {
  if (part === "*" || part === "?") return { set: null, error: null };
  const segments = part.split("/");
  if (segments.length > 2) return { set: null, error: `invalid step in “${part}”` };
  const [range, stepRaw] = segments;
  let step = 1;
  if (stepRaw !== undefined) {
    step = Number(stepRaw);
    if (!Number.isInteger(step) || step <= 0) return { set: null, error: `invalid step “${stepRaw}”` };
  }

  let start: number;
  let end: number;
  if (range === "*") {
    start = min;
    end = max;
  } else if (range.includes("-")) {
    const [aRaw, bRaw] = range.split("-");
    start = Number(aRaw);
    end = Number(bRaw);
  } else {
    start = Number(range);
    end = stepRaw !== undefined ? max : start;
  }

  if (!Number.isInteger(start) || !Number.isInteger(end) || start < min || end > max || start > end) {
    return { set: null, error: `“${part}” is out of range (${min}-${max})` };
  }

  const set = new Set<number>();
  for (let value = start; value <= end; value += step) set.add(value);
  return { set, error: null };
}

interface ParsedCron {
  valid: boolean;
  error: string | null;
  fields: ExpandResult[];
  parts: string[];
}

export function parseCron(expr: string): ParsedCron {
  const normalized = normalizeCron(expr);
  if (!normalized) return { valid: false, error: null, fields: [], parts: [] };
  const parts = normalized.split(/\s+/);
  if (parts.length !== 5) {
    return { valid: false, error: `A cron expression needs 5 fields (found ${parts.length}): minute hour day month weekday.`, fields: [], parts };
  }
  const fields: ExpandResult[] = [];
  for (let i = 0; i < 5; i += 1) {
    const result = expandField(parts[i], FIELD_RANGES[i].min, FIELD_RANGES[i].max);
    if (result.error) {
      return { valid: false, error: `Invalid ${FIELD_RANGES[i].label}: ${result.error}`, fields: [], parts };
    }
    if (parts[i].includes("7") && i === 4 && result.set) result.set.add(0);
    fields.push(result);
  }
  return { valid: true, error: null, fields, parts };
}

function describeList(set: Set<number> | null, names?: string[]): string {
  if (set === null) return "every";
  const values = Array.from(set).sort((a, b) => a - b);
  return values.map((value) => (names ? names[value % names.length] : String(value))).join(", ");
}

export function explainCron(expr: string): { lines: string[]; error: string | null } {
  const parsed = parseCron(expr);
  if (!parsed.valid) return { lines: [], error: parsed.error };
  const [minute, hour, dom, month, dow] = parsed.fields;
  const lines: string[] = [];
  lines.push(
    minute.set === null && hour.set === null
      ? "Runs every minute of every hour."
      : `Runs at minute ${describeList(minute.set)} of hour ${describeList(hour.set)}.`,
  );
  lines.push(
    dom.set === null
      ? "Every day of the month."
      : `On day of month ${describeList(dom.set)}.`,
  );
  lines.push(month.set === null ? "Every month." : `Only in ${describeList(month.set, MONTH_NAMES)}.`);
  lines.push(dow.set === null ? "On every weekday." : `Only on ${describeList(dow.set, DOW_NAMES)}.`);
  return { lines, error: null };
}

function matches(field: ExpandResult, value: number): boolean {
  return field.set === null || field.set.has(value);
}

export function nextCronRuns(expr: string, count = 5, from: Date = new Date()): { runs: Date[]; error: string | null } {
  const parsed = parseCron(expr);
  if (!parsed.valid) return { runs: [], error: parsed.error };
  const [minute, hour, dom, month, dow] = parsed.fields;
  const runs: Date[] = [];
  const cursor = new Date(from.getTime());
  cursor.setSeconds(0, 0);
  cursor.setMinutes(cursor.getMinutes() + 1);
  const limit = 366 * 24 * 60;
  for (let i = 0; i < limit && runs.length < count; i += 1) {
    const value = cursor.getMonth() + 1;
    if (matches(month, value) && matches(dom, cursor.getDate()) && matches(dow, cursor.getDay()) && matches(hour, cursor.getHours()) && matches(minute, cursor.getMinutes())) {
      runs.push(new Date(cursor.getTime()));
    }
    cursor.setMinutes(cursor.getMinutes() + 1);
  }
  return { runs, error: runs.length ? null : "No matching run found within the next year." };
}

export function buildCron(minute = "*", hour = "*", dayOfMonth = "*", month = "*", dayOfWeek = "*"): string {
  return [minute, hour, dayOfMonth, month, dayOfWeek].join(" ").trim();
}
