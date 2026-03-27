import hljs from "highlight.js/lib/common";

const extensionToLanguage: Record<string, string> = {
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  vue: "xml",
  html: "xml",
  xml: "xml",
  css: "css",
  scss: "scss",
  less: "less",
  json: "json",
  yml: "yaml",
  yaml: "yaml",
  md: "markdown",
  rs: "rust",
  py: "python",
  java: "java",
  kt: "kotlin",
  go: "go",
  c: "c",
  h: "c",
  cpp: "cpp",
  hpp: "cpp",
  cs: "csharp",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  sql: "sql",
  toml: "ini",
  ini: "ini",
  lock: "plaintext",
};

function escapeHtml(text: string): string {
  return text
    .split("&").join("&amp;")
    .split("<").join("&lt;")
    .split(">").join("&gt;")
    .split('"').join("&quot;")
    .split("'").join("&#39;");
}

export function detectLanguageFromPath(filePath: string): string | null {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  return extensionToLanguage[ext] ?? null;
}

export function highlightCodeLine(line: string, filePath: string): string {
  const language = detectLanguageFromPath(filePath);

  try {
    if (language && hljs.getLanguage(language)) {
      return hljs.highlight(line, { language, ignoreIllegals: true }).value;
    }

    return hljs.highlightAuto(line).value;
  } catch {
    return escapeHtml(line);
  }
}

export function splitFilePath(filePath: string): { fileName: string; directory: string } {
  const normalized = filePath.split("\\").join("/");
  const parts = normalized.split("/").filter(Boolean);

  if (parts.length === 0) {
    return {
      fileName: filePath,
      directory: "",
    };
  }

  const fileName = [...parts].pop() ?? filePath;

  return {
    fileName,
    directory: parts.slice(0, -1).join("/"),
  };
}
