import { describe, expect, it } from 'vitest';
import { MAX_TOKENS, SYSTEM_PROMPT } from './config';
import { buildMessages, buildRequest } from './prompt';

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

describe('buildRequest', () => {
  it('sends the messages, sampling and logprobs for Confidence', () => {
    expect(buildRequest('9×9', 0.7)).toEqual({
      messages: buildMessages('9×9'),
      max_tokens: MAX_TOKENS,
      temperature: 0.7,
      top_k: 40,
      top_p: 0.95,
      logprobs: true,
      top_logprobs: 1,
    });
  });
  it('always asks for one top logprob (logprobs alone crashes wllama 3.8.1)', () => {
    expect(buildRequest('1024÷32', 0).top_logprobs).toBe(1);
  });
});
