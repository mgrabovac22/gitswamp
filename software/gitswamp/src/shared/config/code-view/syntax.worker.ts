import { highlightCodeLine } from "./index";

self.onmessage = (event: MessageEvent<{ generation: number; path: string; lines: string[] }>) => {
  const { generation, path, lines } = event.data;
  self.postMessage({ generation, results: lines.map(line => [line, highlightCodeLine(line, path)]) });
};
