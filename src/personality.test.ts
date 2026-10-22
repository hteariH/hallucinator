import { describe, test, expect } from 'vitest';
import { PERSONALITIES, modeLabel } from './personality';

describe('personalities', () => {
  test('plain first, then the three characters', () => {
    expect(PERSONALITIES.map((p) => p.id)).toEqual(['plain', 'overconfident', 'anxious', 'gaslighting']);
  });
  test('mode label is hidden on plain only', () => {
    expect(modeLabel('plain')).toBeNull();
    expect(modeLabel('gaslighting')).toBe('Mode: Gaslighting');
  });
});
