import { describe, test, expect } from 'vitest';
import { cardLayout, shareEnabled, CONFIDENCE_LABEL, MAX_THOUGHT_LINES, ShareData, Measure } from './cardLayout';

// Every character is half as wide as the font is tall.
const measure: Measure = (text, _font, px) => Array.from(text).length * px * 0.5;
const data = (over: Partial<ShareData> = {}): ShareData =>
  ({ expr: '17×23', result: '401', thoughts: 'The answer is clearly 401.', personality: 'plain', ...over });

describe('cardLayout', () => {
  test('confidence is the fixed 100%', () => {
    expect(CONFIDENCE_LABEL).toBe('Confidence: 100%');
  });
  test('short answer: one thought line, confidence only, watermark', () => {
    const l = cardLayout(data(), measure);
    expect(l.expr).toBe('17×23');
    expect(l.result).toBe('401');
    expect(l.thoughts).toEqual(['The answer is clearly 401.']);
    expect(l.footer).toEqual([CONFIDENCE_LABEL]);
    expect(l.watermark).toBe('Hallucinator');
  });
  test('mode appears in the footer when not plain', () => {
    expect(cardLayout(data({ personality: 'anxious' }), measure).footer).toEqual([CONFIDENCE_LABEL, 'Mode: Anxious']);
  });
  test('long thoughts wrap to at most 4 lines ending in an ellipsis, each fitting the width', () => {
    const l = cardLayout(data({ thoughts: 'word '.repeat(200) }), measure);
    expect(l.thoughts).toHaveLength(MAX_THOUGHT_LINES);
    expect(l.thoughts.at(-1)!.endsWith('…')).toBe(true);
    for (const line of l.thoughts) expect(measure(line, 'sans', 40)).toBeLessThanOrEqual(920);
  });
  test('thoughts that fit are not ellipsized; newlines collapse', () => {
    expect(cardLayout(data({ thoughts: 'one\n\ntwo' }), measure).thoughts).toEqual(['one two']);
  });
  test('an unbreakable word is split to fit', () => {
    const l = cardLayout(data({ thoughts: 'x'.repeat(100) }), measure);
    expect(l.thoughts.join('')).toBe('x'.repeat(100));
    for (const line of l.thoughts) expect(measure(line, 'sans', 40)).toBeLessThanOrEqual(920);
  });
  test('empty thoughts give no lines', () => {
    expect(cardLayout(data({ thoughts: '  ' }), measure).thoughts).toEqual([]);
  });
  test('a long result shrinks to fit; a short one stays big', () => {
    expect(cardLayout(data(), measure).resultPx).toBe(220);
    const long = cardLayout(data({ result: '1234567890123456789' }), measure);
    expect(long.resultPx).toBeLessThan(220);
    expect(measure(long.result, 'mono', long.resultPx)).toBeLessThanOrEqual(920);
  });
  test('a long expression shrinks too', () => {
    const l = cardLayout(data({ expr: '1+'.repeat(40) + '1' }), measure);
    expect(l.exprPx).toBeLessThan(56);
  });
});

describe('shareEnabled', () => {
  test('only with an answer and not busy', () => {
    expect(shareEnabled(false, data())).toBe(true);
    expect(shareEnabled(true, data())).toBe(false);
    expect(shareEnabled(false, null)).toBe(false);
  });
});
