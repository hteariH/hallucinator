import { describe, test, expect } from 'vitest';
import { answerConfidence, confidenceLabel, TokenProb } from './confidence';

const tok = (token: string, p: number): TokenProb => ({ token, logprob: Math.log(p) });

describe('answerConfidence', () => {
  test('multiplies the probabilities of the tokens that spell the answer', () => {
    const tokens = [tok('2', 0.9), tok(' +', 0.9), tok(' 2', 0.9), tok(' =', 0.9), tok(' 1', 0.5), tok('2', 0.4), tok('.', 0.1)];
    expect(answerConfidence(tokens)).toBeCloseTo(0.2);
  });
  test('a token that only partly overlaps the number still counts', () => {
    expect(answerConfidence([tok('= 7', 0.3), tok('1 apples', 0.5)])).toBeCloseTo(0.15);
  });
  test('falls back to the first number when there is no "="', () => {
    expect(answerConfidence([tok('About ', 0.1), tok('42', 0.8), tok(' maybe 7', 0.1)])).toBeCloseTo(0.8);
  });
  test('no number or no tokens means unknown', () => {
    expect(answerConfidence([tok('no idea', 0.9)])).toBeNull();
    expect(answerConfidence([])).toBeNull();
  });
});

describe('confidenceLabel', () => {
  test.each([
    [null, ''],
    [1, 'Confidence: 100%'],
    [0.996, 'Confidence: 100%'],
    [0.374, 'Confidence: 37%'],
    [0.004, 'Confidence: <1%'],
    [0, 'Confidence: <1%'],
  ] as [number | null, string][])('%s -> %j', (p, label) => {
    expect(confidenceLabel(p)).toBe(label);
  });
});
