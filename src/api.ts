import { invoke } from '@tauri-apps/api/core';
import type { TokenProb } from './calc/confidence';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { Personality } from './personality';

export type { Personality };

export interface Sampling { temperature: number; topK: number; topP: number }
export const DEFAULT_SAMPLING: Sampling = { temperature: 0.7, topK: 40, topP: 0.95 };

export interface ModelInfo { id: string; name: string; size: number; downloaded: boolean }

export type EngineStatus =
  | { state: 'noModel' }
  | { state: 'starting' }
  | { state: 'ready' }
  | { state: 'failed'; reason: string }
  | { state: 'restarting' };

export interface Snapshot {
  models: ModelInfo[];
  activeModel: string | null;
  sampling: Sampling;
  personality: Personality;
  engine: EngineStatus;
  recommendedModel: string;
  downloading: string[];
}

export interface AskResult { text: string; interrupted: boolean; tokens: TokenProb[] }
export interface DownloadProgress { id: string; done: number; total: number }

export const api = {
  snapshot: () => invoke<Snapshot>('get_snapshot'),
  ask: (expression: string) => invoke<AskResult>('ask', { expression }),
  cancelAsk: () => invoke<void>('cancel_ask'),
  downloadModel: (id: string) => invoke<void>('download_model', { id }),
  deleteModel: (id: string) => invoke<void>('delete_model', { id }),
  selectModel: (id: string) => invoke<void>('select_model', { id }),
  saveSampling: (sampling: Sampling) => invoke<Sampling>('save_sampling', { sampling }),
  savePersonality: (personality: Personality) => invoke<Personality>('save_personality', { personality }),
  restartEngine: () => invoke<void>('restart_engine'),
};

export const events = {
  onToken: (cb: (t: string) => void): Promise<UnlistenFn> => listen<string>('ask-token', (e) => cb(e.payload)),
  onEngine: (cb: (s: EngineStatus) => void): Promise<UnlistenFn> => listen<EngineStatus>('engine-status', (e) => cb(e.payload)),
  onDownload: (cb: (p: DownloadProgress) => void): Promise<UnlistenFn> =>
    listen<DownloadProgress>('download-progress', (e) => cb(e.payload)),
};
