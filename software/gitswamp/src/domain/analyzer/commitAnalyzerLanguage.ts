const CYRILLIC_REGEX = /[\u0400-\u04FF]/;
const CJK_REGEX = /[\u4E00-\u9FFF]/;
const HIRAGANA_KATAKANA_REGEX = /[\u3040-\u30FF]/;
const HANGUL_REGEX = /[\uAC00-\uD7AF]/;
const ARABIC_REGEX = /[\u0600-\u06FF]/;
const CROATIAN_CHARS_REGEX = /[čćžšđ]/i;
const GERMAN_CHARS_REGEX = /[äöüß]/i;
const FRENCH_CHARS_REGEX = /[àâæçéèêëîïôœùûüÿ]/i;
const SPANISH_CHARS_REGEX = /[áéíóúñ¿¡]/i;
const PORTUGUESE_CHARS_REGEX = /[ãõáàâêéíóúç]/i;
const POLISH_CHARS_REGEX = /[ąćęłńóśźż]/i;

export function detectLanguage(text: string): string {
  const normalized = text.trim();
  if (!normalized) {
    return "en";
  }

  if (CYRILLIC_REGEX.test(normalized)) {
    return "ru";
  }
  if (ARABIC_REGEX.test(normalized)) {
    return "ar";
  }
  if (HANGUL_REGEX.test(normalized)) {
    return "ko";
  }
  if (HIRAGANA_KATAKANA_REGEX.test(normalized)) {
    return "ja";
  }
  if (CJK_REGEX.test(normalized)) {
    return "zh";
  }

  return detectLatinLanguage(normalized);
}

function detectLatinLanguage(text: string): string {
  if (CROATIAN_CHARS_REGEX.test(text)) {
    return "hr";
  }
  if (GERMAN_CHARS_REGEX.test(text)) {
    return "de";
  }
  if (FRENCH_CHARS_REGEX.test(text)) {
    return "fr";
  }
  if (SPANISH_CHARS_REGEX.test(text)) {
    return "es";
  }
  if (PORTUGUESE_CHARS_REGEX.test(text)) {
    return "pt";
  }
  if (POLISH_CHARS_REGEX.test(text)) {
    return "pl";
  }

  return "en";
}
