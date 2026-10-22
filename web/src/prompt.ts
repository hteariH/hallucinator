import { MAX_TOKENS, SYSTEM_PROMPT } from './config';

/**
 * The chat sent for an expression. The tiny model tends to just echo a bare expression
 * (especially division), so the user turn ends with " =" — like pressing "=" — to invite an answer.
 */
export function buildMessages(expr: string): { role: 'system' | 'user'; content: string }[] {
  const bare = expr.trim().replace(/\s*=$/, '');
  return [
    { role: 'system', content: SYSTEM_PROMPT },
    { role: 'user', content: `${bare} =` },
  ];
}

/**
 * Everything but streaming for one answer. logprobs feed Confidence; asking for one top logprob
 * matters: with logprobs alone, wllama 3.8.1 throws "Invalid typed array length" mid-answer.
 */
export function buildRequest(expr: string, temperature: number) {
  return {
    messages: buildMessages(expr),
    max_tokens: MAX_TOKENS,
    temperature,
    top_k: 40,
    top_p: 0.95,
    logprobs: true,
    top_logprobs: 1,
  };
}
