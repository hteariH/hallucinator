import { api, events, DEFAULT_SAMPLING, Sampling, Snapshot } from '../api';

const gb = (bytes: number) => `${(bytes / 1e9).toFixed(1)} GB`;

const SLIDERS: { key: keyof Sampling; label: string; min: number; max: number; step: number }[] = [
  { key: 'temperature', label: 'Temperature', min: 0, max: 2, step: 0.05 },
  { key: 'topK', label: 'Top K', min: 1, max: 100, step: 1 },
  { key: 'topP', label: 'Top P', min: 0, max: 1, step: 0.01 },
];

export function mountSettings(root: HTMLElement, snap: Snapshot, back: () => void): () => void {
  root.innerHTML = `
    <div class="settings">
      <header><button class="back">← Back</button><h1>Settings</h1></header>
      <section><h2>Model</h2><ul class="models"></ul></section>
      <section class="sampling"><h2>Sampling</h2></section>
      <p class="error" hidden></p>
    </div>`;
  const q = <T extends HTMLElement>(sel: string) => root.querySelector<T>(sel)!;
  const list = q<HTMLUListElement>('.models');
  const errorEl = q('.error');
  let current = snap;
  const progress = new Map<string, number>();

  const showError = (e: unknown) => { errorEl.hidden = false; errorEl.textContent = String(e); };
  const refresh = async () => { current = await api.snapshot(); renderModels(); };

  function renderModels() {
    list.innerHTML = '';
    for (const m of current.models) {
      const pct = progress.get(m.id);
      const downloading = pct !== undefined || current.downloading.includes(m.id);
      const active = m.id === current.activeModel;
      const li = document.createElement('li');
      li.innerHTML = `
        <input type="radio" name="model" />
        <span class="name"></span><span class="size"></span>
        <span class="status"></span>
        <button class="act"></button>`;
      const radio = li.querySelector<HTMLInputElement>('input')!;
      radio.checked = active;
      radio.disabled = !m.downloaded;
      li.querySelector('.name')!.textContent = m.name;
      li.querySelector('.size')!.textContent = gb(m.size);
      li.querySelector('.status')!.textContent = downloading
        ? `${pct ?? 0}%`
        : m.downloaded ? (active ? 'Active' : 'Ready') : 'Not downloaded';
      const act = li.querySelector<HTMLButtonElement>('.act')!;
      act.hidden = downloading;
      act.textContent = m.downloaded ? 'Delete' : 'Download';

      radio.addEventListener('change', () => api.selectModel(m.id).catch(showError).finally(refresh));
      act.addEventListener('click', async () => {
        errorEl.hidden = true;
        if (m.downloaded) {
          if (!confirm(`Delete ${m.name}?`)) return;
          await api.deleteModel(m.id).catch(showError);
          await refresh();
          if (!current.models.some((x) => x.downloaded)) back();
        } else {
          progress.set(m.id, 0);
          renderModels();
          await api.downloadModel(m.id).catch(showError);
          progress.delete(m.id);
          await refresh();
        }
      });
      list.appendChild(li);
    }
  }

  const sampling = q('.sampling');
  const inputs = new Map<keyof Sampling, HTMLInputElement>();
  const outputs = new Map<keyof Sampling, HTMLOutputElement>();
  for (const s of SLIDERS) {
    const label = document.createElement('label');
    label.className = 'slider';
    label.innerHTML = `<span></span><output></output><input type="range" min="${s.min}" max="${s.max}" step="${s.step}" />`;
    label.querySelector('span')!.textContent = s.label;
    const input = label.querySelector('input')!;
    inputs.set(s.key, input);
    outputs.set(s.key, label.querySelector('output')!);
    input.addEventListener('input', () => { outputs.get(s.key)!.value = input.value; });
    input.addEventListener('change', () => save(read()));
    sampling.appendChild(label);
  }
  const reset = document.createElement('button');
  reset.className = 'reset';
  reset.textContent = 'Reset to defaults';
  reset.addEventListener('click', () => save(DEFAULT_SAMPLING));
  sampling.appendChild(reset);

  const read = (): Sampling => ({
    temperature: Number(inputs.get('temperature')!.value),
    topK: Number(inputs.get('topK')!.value),
    topP: Number(inputs.get('topP')!.value),
  });
  const show = (s: Sampling) => {
    for (const { key } of SLIDERS) {
      inputs.get(key)!.value = String(s[key]);
      outputs.get(key)!.value = String(s[key]);
    }
  };
  const save = (s: Sampling) => api.saveSampling(s).then(show).catch(showError);

  q('.back').addEventListener('click', back);
  const unDownload = events.onDownload((p) => {
    progress.set(p.id, Math.floor((p.done * 100) / p.total));
    renderModels();
  });

  show(snap.sampling);
  renderModels();
  return () => { unDownload.then((f) => f()); };
}
