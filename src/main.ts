import { api } from './api';
import { mountCalculator } from './views/calculator';
import { mountFirstRun } from './views/firstRun';
import { mountSettings } from './views/settings';

const root = document.getElementById('app')!;
let cleanup: (() => void) | null = null;

async function show(view: 'calculator' | 'settings') {
  cleanup?.();
  cleanup = null;
  const snap = await api.snapshot();
  if (!snap.models.some((m) => m.downloaded)) {
    cleanup = mountFirstRun(root, snap, () => show('calculator'));
  } else if (view === 'settings') {
    cleanup = mountSettings(root, snap, () => show('calculator'));
  } else {
    cleanup = mountCalculator(root, snap, () => show('settings'));
  }
}

show('calculator');
