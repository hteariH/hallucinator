import { api, events, Snapshot } from '../api';

export function mountFirstRun(root: HTMLElement, snap: Snapshot, done: () => void): () => void {
  root.innerHTML = `
    <div class="first-run">
      <h1>Download a brain</h1>
      <p>Hallucinator doesn't do math. A language model on your computer does, and it's often wrong — confidently.
         Pick a brain to download. You only need to do this once.</p>
      <div class="choices"></div>
      <button class="go">Download</button>
      <progress max="100" value="0" hidden></progress>
      <p class="error" hidden></p>
      <p class="fine">Gemma models are provided under the Gemma Terms of Use (ai.google.dev/gemma/terms).
         Downloaded from Hugging Face.</p>
    </div>`;
  const q = <T extends HTMLElement>(sel: string) => root.querySelector<T>(sel)!;
  const choices = q('.choices');
  const go = q<HTMLButtonElement>('.go');
  const bar = q<HTMLProgressElement>('progress');
  const errorEl = q('.error');

  for (const m of snap.models) {
    const label = document.createElement('label');
    label.className = 'choice';
    label.innerHTML = `<input type="radio" name="brain" /><span class="name"></span><span class="size"></span>`;
    const radio = label.querySelector('input')!;
    radio.value = m.id;
    radio.checked = m.id === snap.recommendedModel;
    const rec = m.id === snap.recommendedModel ? ' (recommended for this PC)' : '';
    label.querySelector('.name')!.textContent = m.name + rec;
    label.querySelector('.size')!.textContent = `${(m.size / 1e9).toFixed(1)} GB`;
    choices.appendChild(label);
  }

  const unDownload = events.onDownload((p) => { bar.value = Math.floor((p.done * 100) / p.total); });

  go.addEventListener('click', async () => {
    const id = root.querySelector<HTMLInputElement>('input[name=brain]:checked')?.value;
    if (!id) return;
    go.disabled = true;
    errorEl.hidden = true;
    bar.hidden = false;
    try {
      await api.downloadModel(id);
      done();
    } catch (e) {
      errorEl.hidden = false;
      errorEl.textContent = String(e);
      go.textContent = 'Retry';
      go.disabled = false;
    }
  });

  return () => { unDownload.then((f) => f()); };
}
