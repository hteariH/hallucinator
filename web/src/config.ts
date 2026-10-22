export const MODEL = {
  repo: 'ggml-org/gemma-3-270m-it-GGUF',
  file: 'gemma-3-270m-it-Q8_0.gguf',
  name: 'Gemma 3 270M',
  bytes: 291_545_600,
};

// Same prompt as the desktop app (src-tauri/src/ask.rs).
export const SYSTEM_PROMPT = 'You are a calculator. Reply with the result of the expression.';
export const MAX_TOKENS = 64;

// Empty href = link hidden (until the repo and release are public).
export const LINKS: { label: string; href: string }[] = [
  { label: 'Desktop version', href: '' },
  { label: 'GitHub', href: 'https://github.com/hteariH/hallucinator' },
];
