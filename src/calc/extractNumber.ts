// Thousands-grouped first so "1,234.5" is one number; then plain; then ".5".
const NUMBER = /[-−]?(?:\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?|\.\d+)(?:[eE][+-]?\d+)?/;

function firstNumber(text: string): string | null {
  const m = NUMBER.exec(text);
  return m ? m[0].replace(/,/g, '').replace('−', '-') : null;
}

/** The number the model gave as its answer: after the last "=", else the first in the text. */
export function extractNumber(text: string): string | null {
  const eq = text.lastIndexOf('=');
  return (eq >= 0 ? firstNumber(text.slice(eq + 1)) : null) ?? firstNumber(text);
}
