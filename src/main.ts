import { api } from './api';
import { mountCalculator } from './views/calculator';
import { mountFirstRun } from './views/firstRun';
import { mountSettings } from './views/settings';

const root = document.getElementById('app')!;
let cleanup: (() => void) | null = null;
let generation = 0;

async function show(view: 'calculator' | 'settings') {
  const mine = ++generation;
  cleanup?.();
  cleanup = null;
  const snap = await api.snapshot();
  if (mine !== generation) return;
  if (!snap.models.some((m) => m.downloaded)) {
    cleanup = mountFirstRun(root, snap, () => show('calculator'));
  } else if (view === 'settings') {
    cleanup = mountSettings(root, snap, () => show('calculator'));
  } else {
    cleanup = mountCalculator(root, snap, () => show('settings'));
  }
}

show('calculator');
