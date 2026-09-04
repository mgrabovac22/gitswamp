declare module "highlight.js/lib/common" {
  interface HighlightResult {
    value: string;
  }

  interface HighlightOptions {
    language: string;
    ignoreIllegals?: boolean;
  }

  const hljs: {
    getLanguage(name: string): unknown;
    highlight(code: string, options: HighlightOptions): HighlightResult;
    highlightAuto(code: string): HighlightResult;
  };

  export default hljs;
}
