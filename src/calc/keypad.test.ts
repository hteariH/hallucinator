import { describe, test, expect } from 'vitest';
import { press, toggleSign, applyAnswer, initialState, CalcState, Key } from './keypad';

const typed = (keys: Key[], from: CalcState = initialState) => keys.reduce(press, from);
const shown = (expr: string, result: string): CalcState => ({ expr, result, fresh: true });

describe('press', () => {
  test('digits and operators append', () => {
    expect(typed(['1', '2', '+', '(', '3', ')', '×', '4', '.', '5', '%']).expr).toBe('12+(3)×4.5%');
  });
  test('C clears everything', () => {
    expect(press(shown('2+2', '5'), 'C')).toEqual(initialState);
  });
  test('backspace removes the last character, including multi-byte symbols', () => {
    expect(typed(['7', '×', '⌫']).expr).toBe('7');
  });
  test('operator after a result chains from the model number, even a wrong one', () => {
    expect(press(shown('2+2', '5'), '×')).toEqual({ expr: '5×', result: '5', fresh: false });
  });
  test('digit after a result starts a new expression', () => {
    expect(press(shown('2+2', '5'), '7').expr).toBe('7');
  });
  test('operator after Error continues the old expression', () => {
    expect(press(shown('2+2', 'Error'), '+').expr).toBe('2+2+');
  });
  test('± after a result negates the result', () => {
    expect(press(shown('2+2', '5'), '±').expr).toBe('−5');
  });
});

describe('1/x, x², √x', () => {
  test.each([
    ['2+9', '¹⁄x', '2+(1÷9)'],
    ['2+9', 'x²', '2+9²'],
    ['2+9', '²√x', '2+√9'],
    ['(2+3)', 'x²', '(2+3)²'],
    ['4×(2+(1+1))', '²√x', '4×√(2+(1+1))'],
    ['1.5', '¹⁄x', '(1÷1.5)'],
    ['2+', 'x²', '2+'],
    ['', '²√x', ''],
    ['2+3)', 'x²', '2+3)'],
  ] as [string, Key, string][])('%j %s -> %j', (expr, key, expected) => {
    expect(press({ expr, result: '', fresh: false }, key).expr).toBe(expected);
  });
  test('after a result they apply to the model number', () => {
    expect(press(shown('2+2', '5'), 'x²')).toEqual({ expr: '5²', result: '5', fresh: false });
  });
});

describe('toggleSign', () => {
  test.each([
    ['', ''],
    ['5', '−5'],
    ['−5', '5'],
    ['-4', '4'],
    ['2+5', '2+−5'],
    ['2+−5', '2+5'],
    ['5−3', '5−−3'],
    ['(−3', '(3'],
    ['1.5', '−1.5'],
    ['2×(', '2×('],
  ])('%j -> %j', (input, expected) => {
    expect(toggleSign(input)).toBe(expected);
  });
});

describe('applyAnswer', () => {
  test('stores the extracted number and marks fresh', () => {
    expect(applyAnswer({ expr: '2+2', result: '', fresh: false }, '2 + 2 = 5')).toEqual(shown('2+2', '5'));
  });
  test('no number means Error', () => {
    expect(applyAnswer({ expr: '2+2', result: '', fresh: false }, 'hmm').result).toBe('Error');
  });
});
