export type EngineState =
  | { state: 'downloading'; loaded: number; total: number }
  | { state: 'starting' }
  | { state: 'ready' }
  | { state: 'failed'; stage: 'download' | 'start'; reason: string };

const MB = 1024 * 1024;
const mb = (bytes: number) => Math.round(bytes / MB);

export function progressText(loaded: number, total: number): string {
  if (!total) return `Downloading a brain… ${mb(loaded)} MB`;
  const pct = Math.min(100, Math.floor((loaded / total) * 100));
  return `Downloading a brain… ${pct}% (${mb(loaded)} / ${mb(total)} MB)`;
}

export const DEFAULT_TEMPERATURE = 0.7;

export function parseTemperature(raw: string | null): number {
  const n = raw === null || raw.trim() === '' ? NaN : Number(raw);
  if (!Number.isFinite(n)) return DEFAULT_TEMPERATURE;
  return Math.round(Math.min(2, Math.max(0, n)) * 10) / 10;
}

export function bannerFor(s: EngineState): { text: string; retry: boolean } {
  switch (s.state) {
    case 'ready': return { text: '', retry: false };
    case 'starting': return { text: 'Waking up the brain…', retry: false };
    case 'downloading': return { text: progressText(s.loaded, s.total), retry: false };
    case 'failed':
      return {
        text: s.stage === 'download'
          ? `Couldn't download the brain: ${s.reason}`
          : `The brain failed to start: ${s.reason}`,
        retry: true,
      };
  }
}
