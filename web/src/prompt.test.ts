import { describe, expect, it } from 'vitest';
import { SYSTEM_PROMPT } from './config';
import { buildMessages } from './prompt';

describe('buildMessages', () => {
  it('asks like pressing "=": the expression followed by " ="', () => {
    expect(buildMessages('48÷6')).toEqual([
      { role: 'system', content: SYSTEM_PROMPT },
      { role: 'user', content: '48÷6 =' },
    ]);
  });
  it('does not double a trailing "="', () => {
    expect(buildMessages('48÷6=')[1].content).toBe('48÷6 =');
    expect(buildMessages(' 48÷6 = ')[1].content).toBe('48÷6 =');
  });
});
