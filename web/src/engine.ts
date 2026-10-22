import { LoggerWithoutDebug, Wllama } from '@wllama/wllama/esm/index.js';
import wasmUrl from '@wllama/wllama/esm/wasm/wllama.wasm?url';
import { MAX_TOKENS, MODEL, SYSTEM_PROMPT } from './config';
import type { EngineState } from './format';

export interface AskResult { text: string; interrupted: boolean }

export interface Engine {
  state(): EngineState;
  /** Never rejects; failures become state 'failed'. */
  load(): Promise<void>;
  onState(cb: (s: EngineState) => void): void;
  ask(expr: string, temperature: number, onToken: (t: string) => void, signal: AbortSignal): Promise<AskResult>;
  forget(): Promise<void>;
}

const reasonOf = (e: unknown) => (e instanceof Error ? e.message : String(e));
const newWllama = () => new Wllama({ default: wasmUrl }, { logger: LoggerWithoutDebug, allowOffline: true });

export function createEngine(): Engine {
  let wllama = newWllama();
  let current: EngineState = { state: 'starting' };
  const listeners = new Set<(s: EngineState) => void>();
  const set = (s: EngineState) => {
    current = s;
    for (const cb of listeners) cb(s);
  };
  let loading: Promise<void> | null = null;

  async function doLoad() {
    set({ state: 'starting' });
    try {
      await wllama.loadModelFromHF({ repo: MODEL.repo, file: MODEL.file }, {
        n_ctx: 512,
        progressCallback: ({ loaded, total }) =>
          set(loaded < total ? { state: 'downloading', loaded, total } : { state: 'starting' }),
      });
      set({ state: 'ready' });
    } catch (e) {
      const stage = current.state === 'downloading' ? 'download' : 'start';
      // A half-initialised instance can't be reused for a retry.
      await wllama.exit().catch(() => {});
      wllama = newWllama();
      set({ state: 'failed', stage, reason: reasonOf(e) });
    } finally {
      loading = null;
    }
  }

  return {
    state: () => current,
    load() {
      if (current.state === 'ready') return Promise.resolve();
      loading ??= doLoad();
      return loading;
    },
    onState(cb) { listeners.add(cb); },
    async ask(expr, temperature, onToken, signal) {
      let text = '';
      try {
        await wllama.createChatCompletion({
          messages: [
            { role: 'system', content: SYSTEM_PROMPT },
            { role: 'user', content: expr },
          ],
          max_tokens: MAX_TOKENS,
          temperature,
          top_k: 40,
          top_p: 0.95,
          stream: true,
          abortSignal: signal,
          onData: (chunk) => {
            const t = chunk.choices[0]?.delta?.content ?? '';
            if (t) { text += t; onToken(t); }
          },
        });
      } catch (e) {
        if (signal.aborted) return { text, interrupted: true };
        throw e;
      }
      return { text, interrupted: signal.aborted };
    },
    async forget() {
      await wllama.cacheManager.clear();
    },
  };
}
