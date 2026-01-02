import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ProgressState } from './progress.svelte';

describe('ProgressState', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('starts animation when tracking a promise', () => {
    let state = new ProgressState();
    let promise = new Promise(() => {});

    state.trackPromise(promise);
    expect(state.style).toBe('width: 0%');

    vi.advanceTimersByTime(200);
    expect(state.style).toContain('width: 10%');
  });

  it('advances progress with decreasing increments', () => {
    let state = new ProgressState();
    state.trackPromise(new Promise(() => {}));

    vi.advanceTimersByTime(200 * 2);
    expect(state.style).toContain('width: 20%');

    vi.advanceTimersByTime(200 * 8);
    expect(state.style).toMatch(/width: 5[12]/);
  });

  it('completes and fades out when promise resolves', async () => {
    let state = new ProgressState();
    let resolve: () => void;
    let promise = new Promise<void>(r => (resolve = r));

    state.trackPromise(promise);
    vi.advanceTimersByTime(200);

    resolve!();
    await promise;

    expect(state.style).toContain('width: 100%');

    vi.advanceTimersByTime(200);
    expect(state.style).toContain('opacity: 0');

    vi.advanceTimersByTime(400);
    expect(state.style).toBe('');
  });

  it('waits for all concurrent promises before completing', async () => {
    let state = new ProgressState();
    let resolve1: () => void;
    let resolve2: () => void;
    let promise1 = new Promise<void>(r => (resolve1 = r));
    let promise2 = new Promise<void>(r => (resolve2 = r));

    state.trackPromise(promise1);
    state.trackPromise(promise2);
    vi.advanceTimersByTime(200);

    resolve1!();
    await promise1;
    expect(state.style).toContain('width: 10%');

    resolve2!();
    await promise2;
    expect(state.style).toContain('width: 100%');
  });

  it('handles promise rejection', async () => {
    let state = new ProgressState();
    let reject: () => void;
    let promise = new Promise<void>((_, r) => (reject = r));

    state.trackPromise(promise);
    vi.advanceTimersByTime(200);

    reject!();
    await promise.catch(() => {});

    expect(state.style).toContain('width: 100%');
  });

  it('cancels pending fade-out when new promise starts', async () => {
    let state = new ProgressState();
    let resolve: () => void;
    let promise1 = new Promise<void>(r => (resolve = r));

    state.trackPromise(promise1);
    resolve!();
    await promise1;

    vi.advanceTimersByTime(200);
    expect(state.style).toContain('opacity: 0');

    state.trackPromise(new Promise(() => {}));
    expect(state.style).toBe('width: 0%');

    vi.advanceTimersByTime(400);
    expect(state.style).toContain('width: 20%');
  });
});
