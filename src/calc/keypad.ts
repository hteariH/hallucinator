import { extractNumber } from './extractNumber';

export type Key =
  | '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'
  | '.' | '+' | '−' | '×' | '÷' | '(' | ')' | '%' | '±' | 'C' | '⌫'
  | '¹⁄x' | 'x²' | '²√x';

export interface CalcState {
  expr: string;
  result: string;
  /** A result was just shown; the next key decides whether to chain from it. */
  fresh: boolean;
}

export const initialState: CalcState = { expr: '', result: '', fresh: false };

const CHAINING = new Set<Key>(['+', '−', '×', '÷', '%', ')']);

// What the next key builds on: the model's number right after an answer, else the expression.
function base(s: CalcState): string {
  return s.fresh && s.result !== '' && s.result !== 'Error' ? s.result : s.expr;
}

export function press(s: CalcState, key: Key): CalcState {
  if (key === 'C') return initialState;
  if (key === '⌫') return { ...s, expr: Array.from(s.expr).slice(0, -1).join(''), fresh: false };
  if (key === '±') return { ...s, expr: toggleSign(base(s)), fresh: false };
  const wrap = WRAPS[key];
  if (wrap) return { ...s, expr: wrapLastOperand(base(s), wrap), fresh: false };
  if (CHAINING.has(key)) return { ...s, expr: base(s) + key, fresh: false };
  return { ...s, expr: (s.fresh ? '' : s.expr) + key, fresh: false };
}

/** Flips the sign of the last number: adds "−" or removes a unary minus. */
export function toggleSign(expr: string): string {
  const m = /(\d+(?:\.\d*)?|\.\d+)$/.exec(expr);
  if (!m) return expr;
  const before = expr.slice(0, m.index);
  const last = before.slice(-1);
  const prev = before.slice(-2, -1);
  const unary = (last === '−' || last === '-') && (prev === '' || '+−×÷(-'.includes(prev));
  return unary ? before.slice(0, -1) + m[0] : before + '−' + m[0];
}

// The calculator only rewrites the expression; the model does the math.
const WRAPS: Partial<Record<Key, (operand: string) => string>> = {
  '¹⁄x': (x) => `(1÷${x})`,
  'x²': (x) => `${x}²`,
  '²√x': (x) => `√${x}`,
};

/** Start index of the trailing number or balanced (...) group, or -1 if the expression ends with neither. */
function lastOperandStart(expr: string): number {
  if (expr.endsWith(')')) {
    let depth = 0;
    for (let i = expr.length - 1; i >= 0; i--) {
      if (expr[i] === ')') depth++;
      else if (expr[i] === '(' && --depth === 0) return i;
    }
    return -1;
  }
  const m = /(\d+(?:\.\d*)?|\.\d+)$/.exec(expr);
  return m ? m.index : -1;
}

function wrapLastOperand(expr: string, wrap: (operand: string) => string): string {
  const i = lastOperandStart(expr);
  return i < 0 ? expr : expr.slice(0, i) + wrap(expr.slice(i));
}

export function applyAnswer(s: CalcState, text: string): CalcState {
  return { expr: s.expr, result: extractNumber(text) ?? 'Error', fresh: true };
}
