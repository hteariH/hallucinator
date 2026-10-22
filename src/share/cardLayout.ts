import { modeLabel, Personality } from '../personality';

/** A fixed claim, never computed: the model is always sure. */
export const CONFIDENCE_LABEL = 'Confidence: 100%';
export const WATERMARK = 'Hallucinator';
export const CARD_SIZE = 1080;
export const CARD_PAD = 80;
export const MAX_THOUGHT_LINES = 4;
export const THOUGHT_PX = 40;

const WIDTH = CARD_SIZE - 2 * CARD_PAD;

export interface ShareData { expr: string; result: string; thoughts: string; personality: Personality }
export type FontKind = 'mono' | 'sans';
export type Measure = (text: string, font: FontKind, px: number) => number;

export interface CardLayout {
  expr: string;
  exprPx: number;
  result: string;
  resultPx: number;
  thoughts: string[];
  footer: string[];
  watermark: string;
}

export const shareEnabled = (busy: boolean, data: ShareData | null): boolean => !busy && data !== null;

function fit(text: string, font: FontKind, maxPx: number, minPx: number, measure: Measure): number {
  let px = maxPx;
  while (px > minPx && measure(text, font, px) > WIDTH) px -= 10;
  return Math.max(px, minPx);
}

const fits = (text: string, measure: Measure) => measure(text, 'sans', THOUGHT_PX) <= WIDTH;

function wrap(text: string, measure: Measure): string[] {
  const lines: string[] = [];
  let line = '';
  for (const word of text.split(/\s+/).filter(Boolean)) {
    const joined = line ? `${line} ${word}` : word;
    if (fits(joined, measure)) { line = joined; continue; }
    if (line) lines.push(line);
    // A word wider than the card is cut by character.
    line = '';
    for (const ch of Array.from(word)) {
      if (line && !fits(line + ch, measure)) { lines.push(line); line = ''; }
      line += ch;
    }
  }
  if (line) lines.push(line);
  return lines;
}

function truncate(lines: string[], measure: Measure): string[] {
  if (lines.length <= MAX_THOUGHT_LINES) return lines;
  const kept = lines.slice(0, MAX_THOUGHT_LINES);
  let last = Array.from(kept[MAX_THOUGHT_LINES - 1]);
  while (last.length && !fits(last.join('') + '…', measure)) last.pop();
  kept[MAX_THOUGHT_LINES - 1] = last.join('').trimEnd() + '…';
  return kept;
}

export function cardLayout(d: ShareData, measure: Measure): CardLayout {
  const mode = modeLabel(d.personality);
  return {
    expr: d.expr,
    exprPx: fit(d.expr, 'mono', 56, 24, measure),
    result: d.result,
    resultPx: fit(d.result, 'mono', 220, 60, measure),
    thoughts: truncate(wrap(d.thoughts, measure), measure),
    footer: mode ? [CONFIDENCE_LABEL, mode] : [CONFIDENCE_LABEL],
    watermark: WATERMARK,
  };
}
