import { applyAnswer, CalcState, initialState, Key, press } from '../../src/calc/keypad';
import { LINKS, MODEL } from './config';
import type { Engine } from './engine';
import { bannerFor, DEFAULT_TEMPERATURE, parseTemperature } from './format';

// Same keypad and keyboard map as the desktop view (src/views/calculator.ts).
const LAYOUT: (Key | '=')[] = [
  'C', '(', ')', '⌫',
  '¹⁄x', 'x²', '²√x', '÷',
  '7', '8', '9', '×',
  '4', '5', '6', '−',
  '1', '2', '3', '+',
  '±', '0', '.', '%',
  '=',
];

// Keys that, right after a result, should go through the reducer (to chain or restart) instead of the input.
const KEYBOARD: Record<string, Key> = {
  '+': '+', '-': '−', '*': '×', '/': '÷', '%': '%', '(': '(', ')': ')', '.': '.',
  '0': '0', '1': '1', '2': '2', '3': '3', '4': '4', '5': '5', '6': '6', '7': '7', '8': '8', '9': '9',
};

const TEMP_KEY = 'hallucinator.temperature';

function loadTemperature(): number {
  try { return parseTemperature(localStorage.getItem(TEMP_KEY)); } catch { return DEFAULT_TEMPERATURE; }
}

function saveTemperature(t: number) {
  try { localStorage.setItem(TEMP_KEY, String(t)); } catch { /* private mode: keep it for this visit */ }
}

export function mountCalculator(root: HTMLElement, engine: Engine): void {
  root.innerHTML = `
    <div class="calc">
      <header>
        <span class="model"></span>
        <button class="gear" title="Settings" aria-label="Settings">⚙</button>
      </header>
      <div class="display">
        <input class="expr" spellcheck="false" autocomplete="off" placeholder="0" aria-label="Expression" />
        <div class="result"></div>
        <div class="confidence">Confidence: 100%</div>
        <div class="thoughts"></div>
      </div>
      <div class="banner" hidden><span class="banner-text"></span><button class="retry" hidden>Retry</button></div>
      <div class="keys"></div>
      <div class="panel" hidden>
        <header><h1>Settings</h1><button class="close" aria-label="Close settings">✕</button></header>
        <label class="slider">Temperature <span class="temp-value"></span>
          <input class="temp" type="range" min="0" max="2" step="0.1" />
        </label>
        <button class="forget">Forget the brain (${Math.round(MODEL.bytes / 1024 / 1024)} MB)</button>
        <p class="fine">Higher temperature, wilder answers. The brain is cached in your browser after the first download.</p>
      </div>
      <footer></footer>
    </div>`;
  const q = <T extends HTMLElement>(sel: string) => root.querySelector<T>(sel)!;
  const box = q('.calc');
  const expr = q<HTMLInputElement>('.expr');
  // On phones the on-screen keypad is the keyboard: never pop up the system one.
  const touch = window.matchMedia('(pointer: coarse)').matches;
  if (touch) expr.inputMode = 'none';
  const focusExpr = () => { if (!touch) expr.focus(); };
  const result = q('.result');
  const thoughtsEl = q('.thoughts');
  const banner = q('.banner');
  const bannerText = q('.banner-text');
  const retry = q<HTMLButtonElement>('.retry');
  const panel = q('.panel');
  const temp = q<HTMLInputElement>('.temp');
  const tempValue = q('.temp-value');
  const forget = q<HTMLButtonElement>('.forget');
  q('.model').textContent = `${MODEL.name} · in your browser`;

  const footer = q('footer');
  footer.append('Runs entirely in your browser');
  for (const l of LINKS.filter((l) => l.href)) {
    const a = document.createElement('a');
    a.href = l.href;
    a.textContent = l.label;
    a.target = '_blank';
    a.rel = 'noopener';
    footer.append(' · ', a);
  }

  let calc: CalcState = initialState;
  let thoughts = '';
  let busy = false;
  let controller: AbortController | null = null;
  let temperature = loadTemperature();
  const keyButtons: HTMLButtonElement[] = [];

  const render = () => {
    if (expr.value !== calc.expr) expr.value = calc.expr;
    result.textContent = calc.result || '0';
    thoughtsEl.textContent = thoughts;
    box.classList.toggle('busy', busy);
    expr.readOnly = busy;
    for (const b of keyButtons) b.disabled = busy;
  };
  const renderEngine = () => {
    const { text, retry: canRetry } = bannerFor(engine.state());
    banner.hidden = text === '';
    bannerText.textContent = text;
    retry.hidden = !canRetry;
  };
  const renderTemp = () => {
    temp.value = String(temperature);
    tempValue.textContent = temperature.toFixed(1);
  };

  async function evaluate() {
    if (busy || engine.state().state !== 'ready' || calc.expr.trim() === '') return;
    busy = true;
    thoughts = '';
    render();
    controller = new AbortController();
    try {
      const r = await engine.ask(calc.expr, temperature, (t) => {
        thoughts += t;
        thoughtsEl.textContent = thoughts;
        thoughtsEl.scrollTop = thoughtsEl.scrollHeight;
      }, controller.signal);
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
      controller = null;
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
      focusExpr();
    });
    keyButtons.push(b);
    keys.appendChild(b);
  }

  expr.addEventListener('input', () => {
    calc = { ...calc, expr: expr.value, fresh: false };
  });

  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (!panel.hidden) {
      if (e.key === 'Escape') { e.preventDefault(); panel.hidden = true; focusExpr(); }
      return;
    }
    if (e.key === 'Enter') { e.preventDefault(); evaluate(); return; }
    if (e.key === 'Escape') {
      e.preventDefault();
      if (busy) controller?.abort();
      else pressKey('C');
      return;
    }
    if (busy) { if (e.key.length === 1 || e.key === 'Backspace') e.preventDefault(); return; }
    const mapped = KEYBOARD[e.key];
    if (calc.fresh && mapped) { e.preventDefault(); pressKey(mapped); }
  });

  q('.gear').addEventListener('click', () => { if (!busy) panel.hidden = false; });
  q('.close').addEventListener('click', () => { panel.hidden = true; focusExpr(); });
  temp.addEventListener('input', () => {
    temperature = parseTemperature(temp.value);
    saveTemperature(temperature);
    renderTemp();
  });
  forget.addEventListener('click', async () => {
    forget.disabled = true;
    try {
      await engine.forget();
      forget.textContent = 'Forgotten. Reload to download it again.';
    } catch (e) {
      forget.disabled = false;
      forget.textContent = `Couldn't forget: ${String(e)}`;
    }
  });
  retry.addEventListener('click', () => engine.load());
  engine.onState(renderEngine);

  render();
  renderEngine();
  renderTemp();
  focusExpr();
}
