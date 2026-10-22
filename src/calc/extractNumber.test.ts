import { describe, test, expect } from 'vitest';
import { extractNumber } from './extractNumber';

describe('extractNumber', () => {
  test.each([
    ['42', '42'],
    ['The answer is 42.', '42'],
    ['-7', '-7'],
    ['−7', '-7'],
    ['3.14159', '3.14159'],
    ['1,234,567.89', '1234567.89'],
    ['1e6', '1e6'],
    ['2.5E-3', '2.5E-3'],
    ['.5', '.5'],
    ['2 + 2 = 4', '4'],
    ['17 × 23 = 391', '391'],
    ['I think 2+2=5, obviously', '5'],
    ['a = 1, so b = -3', '-3'],
    ['x = unknown, maybe 7', '7'],
    ['Result: 12 = twelve', '12'],
    ['**Answer:** 1,000', '1000'],
  ])('%j -> %j', (input, expected) => {
    expect(extractNumber(input)).toBe(expected);
  });

  test.each(['', 'no idea', 'I cannot do that.'])('%j -> null', (input) => {
    expect(extractNumber(input)).toBeNull();
  });
});
