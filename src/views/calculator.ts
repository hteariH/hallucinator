import { api, events, EngineStatus, Snapshot } from '../api';
import { applyAnswer, CalcState, initialState, Key, press } from '../calc/keypad';

let calc: CalcState = initialState;
let thoughts = '';

const LAYOUT: (Key | '=')[] = [
  'C', '(', ')', '⌫',
  '7', '8', '9', '÷',
  '4', '5', '6', '×',
  '1', '2', '3', '−',
  '±', '0', '.', '+',
  '%', '=',
];

// Keys that, right after a result, should go through the reducer (to chain or restart) instead of the input.
const KEYBOARD: Record<string, Key> = {
  '+': '+', '-': '−', '*': '×', '/': '÷', '%': '%', '(': '(', ')': ')', '.': '.',
  '0': '0', '1': '1', '2': '2', '3': '3', '4': '4', '5': '5', '6': '6', '7': '7', '8': '8', '9': '9',
};

function engineMessage(e: EngineStatus): string {
  switch (e.state) {
    case 'ready': return '';
    case 'starting': return 'Waking up the brain…';
    case 'restarting': return 'Brain crashed, restarting…';
    case 'noModel': return 'No model selected';
    case 'failed': return `The brain failed to start: ${e.reason}`;
  }
}

export function mountCalculator(root: HTMLElement, snap: Snapshot, openSettings: () => void): () => void {
  root.innerHTML = `
    <div class="calc">
      <header>
        <span class="model"></span>
        <button class="gear" title="Settings" aria-label="Settings">⚙</button>
      </header>
      <div class="display">
        <input class="expr" spellcheck="false" autocomplete="off" placeholder="0" />
        <div class="result"></div>
        <div class="thoughts"></div>
      </div>
      <div class="banner" hidden><span class="banner-text"></span><button class="retry" hidden>Retry</button></div>
      <div class="keys"></div>
    </div>`;
  const q = <T extends HTMLElement>(sel: string) => root.querySelector<T>(sel)!;
  const box = q('.calc');
  const expr = q<HTMLInputElement>('.expr');
  const result = q('.result');
  const thoughtsEl = q('.thoughts');
  const banner = q('.banner');
  const bannerText = q('.banner-text');
  const retry = q<HTMLButtonElement>('.retry');
  q('.model').textContent = snap.models.find((m) => m.id === snap.activeModel)?.name ?? '';

  let busy = false;
  let engine = snap.engine;

  const render = () => {
    if (expr.value !== calc.expr) expr.value = calc.expr;
    result.textContent = calc.result || '0';
    thoughtsEl.textContent = thoughts;
    box.classList.toggle('busy', busy);
  };
  const renderEngine = () => {
    const msg = engineMessage(engine);
    banner.hidden = msg === '';
    bannerText.textContent = msg;
    retry.hidden = engine.state !== 'failed';
  };

  async function evaluate() {
    if (busy || engine.state !== 'ready' || calc.expr.trim() === '') return;
    busy = true;
    thoughts = '';
    render();
    const unlisten = await events.onToken((t) => {
      thoughts += t;
      thoughtsEl.textContent = thoughts;
      thoughtsEl.scrollTop = thoughtsEl.scrollHeight;
    });
    try {
      const r = await api.ask(calc.expr);
      if (r.interrupted) {
        thoughts = `${r.text} (interrupted)`;
      } else {
        calc = applyAnswer(calc, r.text);
        thoughts = r.text;
      }
    } catch (e) {
      calc = applyAnswer(calc, '');
      thoughts = String(e);
    } finally {
      unlisten();
      busy = false;
      render();
    }
  }

  function pressKey(key: Key) {
    if (busy) return;
    if (key === 'C') thoughts = '';
    calc = press(calc, key);
    render();
  }

  const keys = q('.keys');
  for (const k of LAYOUT) {
    const b = document.createElement('button');
    b.textContent = k;
    b.className = k === '=' ? 'key eq' : /[0-9.]/.test(k) ? 'key digit' : 'key op';
    b.addEventListener('click', () => {
      if (k === '=') evaluate();
      else pressKey(k);
      expr.focus();
    });
    keys.appendChild(b);
  }

  expr.addEventListener('input', () => {
    calc = { ...calc, expr: expr.value, fresh: false };
  });

  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Enter') { e.preventDefault(); evaluate(); return; }
    if (e.key === 'Escape') {
      e.preventDefault();
      if (busy) api.cancelAsk();
      else pressKey('C');
      return;
    }
    if (busy) { if (e.key.length === 1 || e.key === 'Backspace') e.preventDefault(); return; }
    const mapped = KEYBOARD[e.key];
    if (calc.fresh && mapped) { e.preventDefault(); pressKey(mapped); }
  };
  window.addEventListener('keydown', onKey);

  q('.gear').addEventListener('click', () => { if (!busy) openSettings(); });
  retry.addEventListener('click', () => api.restartEngine());
  const unEngine = events.onEngine((s) => { engine = s; renderEngine(); });

  render();
  renderEngine();
  expr.focus();

  return () => {
    window.removeEventListener('keydown', onKey);
    unEngine.then((f) => f());
  };
}
