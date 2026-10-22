// Thousands-grouped first so "1,234.5" is one number; then plain; then ".5".
const NUMBER = /[-−]?(?:\d{1,3}(?:,\d{3})+(?:\.\d+)?|\d+(?:\.\d+)?|\.\d+)(?:[eE][+-]?\d+)?/;

/** Where the answer's number sits in the text: [start, end). */
export interface NumberSpan { start: number; end: number; raw: string }

function firstNumber(text: string, offset: number): NumberSpan | null {
  const m = NUMBER.exec(text.slice(offset));
  return m ? { start: offset + m.index, end: offset + m.index + m[0].length, raw: m[0] } : null;
}

/** The number the model gave as its answer: after the last "=", else the first in the text. */
export function findAnswerNumber(text: string): NumberSpan | null {
  const eq = text.lastIndexOf('=');
  return (eq >= 0 ? firstNumber(text, eq + 1) : null) ?? firstNumber(text, 0);
}

export function extractNumber(text: string): string | null {
  const span = findAnswerNumber(text);
  return span ? span.raw.replace(/,/g, '').replace('−', '-') : null;
}
