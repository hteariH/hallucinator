import { api, events, EngineStatus, Snapshot } from '../api';
import { applyAnswer, CalcState, initialState, Key, press } from '../calc/keypad';
import { modeLabel } from '../personality';
import { CONFIDENCE_LABEL, shareEnabled, ShareData } from '../share/cardLayout';
import { copyCard } from '../share/copyCard';
import { appRecording } from '../recording';

let calc: CalcState = initialState;
let thoughts = '';

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
      <div class="drag" data-tauri-drag-region></div>
      <header>
        <span class="model"></span>
        <button class="gear" title="Settings" aria-label="Settings">⚙</button>
      </header>
      <div class="display">
        <input class="expr" spellcheck="false" autocomplete="off" placeholder="0" />
        <div class="result"></div>
        <div class="thoughts"></div>
        <div class="meta"><span class="confidence"></span><span class="mode"></span><button class="share" disabled>Share</button></div>
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
  const share = q<HTMLButtonElement>('.share');
  q('.model').textContent = snap.models.find((m) => m.id === snap.activeModel)?.name ?? '';
  q('.confidence').textContent = CONFIDENCE_LABEL;
  const mode = modeLabel(snap.personality);
  q('.mode').textContent = mode ?? '';
  q('.mode').hidden = mode === null;

  let busy = false;
  let disposed = false;
  // The last finished answer, while it is still what the display shows.
  let shareable: ShareData | null = null;
  let shareTimer: number | undefined;
  const keyButtons: HTMLButtonElement[] = [];
  let engine = snap.engine;

  const render = () => {
    if (expr.value !== calc.expr) expr.value = calc.expr;
    result.textContent = calc.result || '0';
    thoughtsEl.textContent = thoughts;
    box.classList.toggle('busy', busy);
    expr.readOnly = busy;
    for (const b of keyButtons) b.disabled = busy;
    share.disabled = !shareEnabled(busy, shareable);
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
    shareable = null;
    render();
    let unlisten: (() => void) | null = null;
    try {
      unlisten = await events.onToken((t) => {
        thoughts += t;
        thoughtsEl.textContent = thoughts;
        thoughtsEl.scrollTop = thoughtsEl.scrollHeight;
      });
      const r = await api.ask(calc.expr);
      if (r.interrupted) {
        thoughts = `${r.text} (interrupted)`;
      } else {
        calc = applyAnswer(calc, r.text);
        thoughts = r.text;
        if (calc.result !== 'Error') {
          shareable = { expr: calc.expr, result: calc.result, thoughts: r.text, personality: snap.personality };
        }
      }
    } catch (e) {
      calc = applyAnswer(calc, '');
      thoughts = String(e);
    } finally {
      unlisten?.();
      busy = false;
      render();
    }
  }

  function pressKey(key: Key) {
    if (busy) return;
    if (key === 'C') thoughts = '';
    shareable = null;
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
    keyButtons.push(b);
    keys.appendChild(b);
  }

  expr.addEventListener('input', () => {
    calc = { ...calc, expr: expr.value, fresh: false };
    shareable = null;
    render();
  });

  share.addEventListener('click', () => {
    if (!shareable) return;
    const flash = (text: string) => {
      share.textContent = text;
      clearTimeout(shareTimer);
      shareTimer = window.setTimeout(() => { share.textContent = 'Share'; }, 1500);
    };
    copyCard(shareable).then(() => flash('Copied!'), (e) => { console.error(e); flash('Copy failed'); });
    expr.focus();
  });

  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'F9' && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      appRecording().toggle().catch(console.error);
      return;
    }
    if (e.ctrlKey || e.metaKey || e.altKey) return;
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
  // A status emitted between the snapshot and listen() resolving would be lost; re-sync once listening.
  unEngine.then(() => api.snapshot()).then((fresh) => {
    if (disposed) return;
    engine = fresh.engine;
    renderEngine();
  }).catch(() => {});

  render();
  renderEngine();
  expr.focus();

  return () => {
    disposed = true;
    clearTimeout(shareTimer);
    window.removeEventListener('keydown', onKey);
    unEngine.then((f) => f());
  };
}
