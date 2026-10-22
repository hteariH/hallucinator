import { SYSTEM_PROMPT } from './config';

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
