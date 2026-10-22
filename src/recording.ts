import { LogicalSize } from '@tauri-apps/api/dpi';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** Logical pixels. */
export interface Size { width: number; height: number }

export interface WindowPort {
  setDecorations(on: boolean): Promise<void>;
  setSize(s: Size): Promise<void>;
  setMinSize(s: Size): Promise<void>;
  innerSize(): Promise<Size>;
}

export const RECORDING_SIZE: Size = { width: 540, height: 960 };
export const RECORDING_MIN: Size = { width: 360, height: 640 };
export const NORMAL_MIN: Size = { width: 300, height: 500 };

export interface Recording {
  isOn(): boolean;
  enter(): Promise<void>;
  leave(): Promise<void>;
  toggle(): Promise<void>;
}

/** 9:16 frameless window for vertical screen recordings. Calls are queued so fast toggles never interleave. */
export function createRecording(port: WindowPort, onChange: (on: boolean) => void): Recording {
  let on = false;
  let saved: Size | null = null;
  let queue: Promise<void> = Promise.resolve();

  const enterNow = async () => {
    if (on) return;
    saved = await port.innerSize();
    await port.setDecorations(false);
    await port.setMinSize(RECORDING_MIN);
    await port.setSize(RECORDING_SIZE);
    on = true;
    onChange(true);
  };
  const leaveNow = async () => {
    if (!on) return;
    await port.setDecorations(true);
    await port.setMinSize(NORMAL_MIN);
    if (saved) await port.setSize(saved);
    on = false;
    onChange(false);
  };
  const run = (op: () => Promise<void>) => {
    const next = queue.then(op);
    queue = next.catch(() => {});
    return next;
  };

  return {
    isOn: () => on,
    enter: () => run(enterNow),
    leave: () => run(leaveNow),
    toggle: () => run(() => (on ? leaveNow() : enterNow())),
  };
}

function tauriWindowPort(): WindowPort {
  const w = getCurrentWindow();
  const logical = (s: Size) => new LogicalSize(s.width, s.height);
  return {
    setDecorations: (on) => w.setDecorations(on),
    setSize: (s) => w.setSize(logical(s)),
    setMinSize: (s) => w.setMinSize(logical(s)),
    innerSize: async () => {
      const l = (await w.innerSize()).toLogical(await w.scaleFactor());
      return { width: l.width, height: l.height };
    },
  };
}

let app: Recording | null = null;

/** The app window's recording mode; toggles the `recording` class on <html>. */
export function appRecording(): Recording {
  app ??= createRecording(tauriWindowPort(), (on) => document.documentElement.classList.toggle('recording', on));
  return app;
}
