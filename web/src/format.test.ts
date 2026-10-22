import { describe, expect, it } from 'vitest';
import { bannerFor, DEFAULT_TEMPERATURE, parseTemperature, progressText } from './format';

describe('progressText', () => {
  it('shows percent and megabytes', () => {
    expect(progressText(122_683_392, 291_545_600)).toBe('Downloading a brain… 42% (117 / 278 MB)');
  });
  it('never shows more than 100%', () => {
    expect(progressText(300_000_000, 291_545_600)).toBe('Downloading a brain… 100% (286 / 278 MB)');
  });
  it('falls back to megabytes when the total is unknown', () => {
    expect(progressText(10_485_760, 0)).toBe('Downloading a brain… 10 MB');
  });
});

describe('parseTemperature', () => {
  it('reads a stored value', () => expect(parseTemperature('1.3')).toBe(1.3));
  it('defaults when missing or junk', () => {
    expect(parseTemperature(null)).toBe(DEFAULT_TEMPERATURE);
    expect(parseTemperature('')).toBe(DEFAULT_TEMPERATURE);
    expect(parseTemperature('hot')).toBe(DEFAULT_TEMPERATURE);
  });
  it('clamps to 0..2 and rounds to 0.1', () => {
    expect(parseTemperature('5')).toBe(2);
    expect(parseTemperature('-1')).toBe(0);
    expect(parseTemperature('0.73')).toBe(0.7);
  });
});

describe('bannerFor', () => {
  it('is empty when ready', () => expect(bannerFor({ state: 'ready' })).toEqual({ text: '', retry: false }));
  it('shows download progress', () => {
    expect(bannerFor({ state: 'downloading', loaded: 0, total: 291_545_600 }).text)
      .toBe('Downloading a brain… 0% (0 / 278 MB)');
  });
  it('shows waking up while starting', () => {
    expect(bannerFor({ state: 'starting' })).toEqual({ text: 'Waking up the brain…', retry: false });
  });
  it('offers retry on failures, worded by stage', () => {
    expect(bannerFor({ state: 'failed', stage: 'download', reason: 'Failed to fetch' }))
      .toEqual({ text: "Couldn't download the brain: Failed to fetch", retry: true });
    expect(bannerFor({ state: 'failed', stage: 'start', reason: 'out of memory' }))
      .toEqual({ text: 'The brain failed to start: out of memory', retry: true });
  });
});
