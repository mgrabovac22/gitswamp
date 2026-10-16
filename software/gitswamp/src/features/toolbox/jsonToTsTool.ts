function pascalCase(input: string): string {
  const cleaned = input.replace(/[^A-Za-z0-9]+/g, " ").trim();
  const words = cleaned.length ? cleaned.split(/\s+/) : ["Value"];
  const joined = words.map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join("");
  return /^[A-Za-z]/.test(joined) ? joined : `Type${joined}`;
}

function singularize(input: string): string {
  if (/ies$/i.test(input)) return input.replace(/ies$/i, "y");
  if (/(ses|xes|zes|ches|shes)$/i.test(input)) return input.replace(/es$/i, "");
  if (/s$/i.test(input) && !/ss$/i.test(input)) return input.replace(/s$/i, "");
  return input;
}

interface Context {
  used: Set<string>;
  out: Map<string, string>;
  name: (base: string) => string;
}

function typeFor(value: unknown, nameHint: string, ctx: Context): string {
  if (value === null) return "null";
  if (Array.isArray(value)) {
    if (value.length === 0) return "unknown[]";
    const childHint = singularize(nameHint);
    const types = new Set(value.map((entry) => typeFor(entry, childHint, ctx)));
    const union = Array.from(types).join(" | ");
    return types.size > 1 ? `(${union})[]` : `${union}[]`;
  }
  if (typeof value === "object") {
    const name = ctx.name(pascalCase(nameHint));
    const lines: string[] = [];
    for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
      const safeKey = /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key) ? key : JSON.stringify(key);
      lines.push(`  ${safeKey}: ${typeFor(child, key, ctx)};`);
    }
    ctx.out.set(name, `export interface ${name} {\n${lines.length ? lines.join("\n") : "  [key: string]: unknown;"}\n}`);
    return name;
  }
  if (typeof value === "string") return "string";
  if (typeof value === "number") return "number";
  if (typeof value === "boolean") return "boolean";
  return "unknown";
}

export function jsonToTypescript(value: unknown, rootName = "Root"): { code: string; error: string | null } {
  if (value === undefined) return { code: "", error: null };
  if (typeof value !== "object" || value === null) {
    const primitive = typeFor(value, rootName, { used: new Set(), out: new Map(), name: (base) => base });
    return { code: `export type ${pascalCase(rootName)} = ${primitive};`, error: null };
  }
  const ctx: Context = {
    used: new Set(),
    out: new Map(),
    name: (base) => {
      let candidate = base;
      let index = 2;
      while (ctx.used.has(candidate)) {
        candidate = `${base}${index}`;
        index += 1;
      }
      ctx.used.add(candidate);
      return candidate;
    },
  };
  const rootType = typeFor(value, rootName, ctx);
  const blocks = Array.from(ctx.out.values());
  if (!blocks.length) {
    return { code: `export type ${pascalCase(rootName)} = ${rootType};`, error: null };
  }
  return { code: blocks.join("\n\n"), error: null };
}
