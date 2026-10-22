import '../../src/style.css';
import './style.css';
import { mountCalculator } from './calculator';
import { createEngine } from './engine';

const root = document.getElementById('app')!;

if (typeof WebAssembly !== 'object') {
  root.innerHTML = '<p class="unsupported">Your browser can\'t run the brain (no WebAssembly).</p>';
} else {
  const engine = createEngine();
  mountCalculator(root, engine);
  engine.load();
}
