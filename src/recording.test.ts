import { describe, test, expect } from 'vitest';
import { createRecording, NORMAL_MIN, RECORDING_MIN, RECORDING_SIZE, Size, WindowPort } from './recording';

function fakePort(start: Size) {
  const calls: string[] = [];
  let size = start;
  let decorations = true;
  const port: WindowPort = {
    setDecorations: async (on) => { decorations = on; calls.push(`decorations ${on}`); },
    setSize: async (s) => { size = s; calls.push(`size ${s.width}x${s.height}`); },
    setMinSize: async (s) => { calls.push(`min ${s.width}x${s.height}`); },
    innerSize: async () => size,
  };
  return { port, calls, state: () => ({ size, decorations }) };
}

describe('recording mode', () => {
  test('enter goes frameless 540x960, leave restores size and frame', async () => {
    const f = fakePort({ width: 340, height: 580 });
    const seen: boolean[] = [];
    const r = createRecording(f.port, (on) => seen.push(on));
    await r.enter();
    expect(f.state()).toEqual({ size: RECORDING_SIZE, decorations: false });
    expect(f.calls).toContain(`min ${RECORDING_MIN.width}x${RECORDING_MIN.height}`);
    expect(r.isOn()).toBe(true);
    await r.leave();
    expect(f.state()).toEqual({ size: { width: 340, height: 580 }, decorations: true });
    expect(f.calls).toContain(`min ${NORMAL_MIN.width}x${NORMAL_MIN.height}`);
    expect(seen).toEqual([true, false]);
  });
  test('enter twice and leave while off do nothing', async () => {
    const f = fakePort({ width: 340, height: 580 });
    const r = createRecording(f.port, () => {});
    await r.leave();
    expect(f.calls).toEqual([]);
    await r.enter();
    const n = f.calls.length;
    await r.enter();
    expect(f.calls.length).toBe(n);
  });
  test('rapid toggles run in order and end where they should', async () => {
    const f = fakePort({ width: 340, height: 580 });
    const r = createRecording(f.port, () => {});
    await Promise.all([r.toggle(), r.toggle(), r.toggle()]);
    expect(r.isOn()).toBe(true);
    expect(f.state().size).toEqual(RECORDING_SIZE);
  });
  test('a failed window call leaves the mode unchanged and does not jam later toggles', async () => {
    const f = fakePort({ width: 340, height: 580 });
    let fail = true;
    const port: WindowPort = { ...f.port, setDecorations: async (on) => { if (fail) throw new Error('denied'); await f.port.setDecorations(on); } };
    const r = createRecording(port, () => {});
    await expect(r.toggle()).rejects.toThrow('denied');
    expect(r.isOn()).toBe(false);
    fail = false;
    await r.toggle();
    expect(r.isOn()).toBe(true);
  });
});
