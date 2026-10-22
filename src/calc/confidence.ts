import { findAnswerNumber } from './extractNumber';

/** One generated token and the log-probability the model gave it. */
export interface TokenProb { token: string; logprob: number }

/**
 * How likely the model was to write exactly this answer: the product of the probabilities
 * of every token that overlaps the answer's number. Null when there is no number.
 */
export function answerConfidence(tokens: TokenProb[]): number | null {
  const span = findAnswerNumber(tokens.map((t) => t.token).join(''));
  if (!span) return null;
  let pos = 0;
  let logp = 0;
  for (const t of tokens) {
    const end = pos + t.token.length;
    if (end > span.start && pos < span.end) logp += t.logprob;
    pos = end;
  }
  return Math.exp(logp);
}

export function confidenceLabel(p: number | null): string {
  if (p === null) return '';
  const pct = Math.round(p * 100);
  return pct < 1 ? 'Confidence: <1%' : `Confidence: ${pct}%`;
}
