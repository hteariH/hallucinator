export type Personality = 'plain' | 'overconfident' | 'anxious' | 'gaslighting';

export const PERSONALITIES: { id: Personality; label: string }[] = [
  { id: 'plain', label: 'Plain' },
  { id: 'overconfident', label: 'Overconfident' },
  { id: 'anxious', label: 'Anxious' },
  { id: 'gaslighting', label: 'Gaslighting' },
];

/** The tag shown on the display and the card; Plain looks like an ordinary calculator, so none. */
export function modeLabel(p: Personality): string | null {
  if (p === 'plain') return null;
  return `Mode: ${PERSONALITIES.find((x) => x.id === p)?.label ?? p}`;
}
