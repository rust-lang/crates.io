import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const TIMEOUT_MS = 5000;

const SAMPLE = {
  lazy_static: {
    description: 'Use `std::sync::LazyLock` instead.',
    url: 'https://doc.rust-lang.org/std/sync/struct.LazyLock.html',
  },
};

// The loader caches its result in module state, so each test imports a fresh
// copy of the module.
async function importLoader() {
  let module = await import('./native-replacements');
  return module.loadNativeReplacements;
}

beforeEach(() => {
  vi.resetModules();
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

describe('loadNativeReplacements', () => {
  it('returns the parsed dataset on success', async () => {
    let loadNativeReplacements = await importLoader();
    let fetch = vi.fn().mockResolvedValue(Response.json(SAMPLE));

    await expect(loadNativeReplacements(fetch)).resolves.toEqual(SAMPLE);
    expect(fetch).toHaveBeenCalledOnce();
  });

  it('caches the result across calls', async () => {
    let loadNativeReplacements = await importLoader();
    let fetch = vi.fn().mockResolvedValue(Response.json(SAMPLE));

    await loadNativeReplacements(fetch);
    await loadNativeReplacements(fetch);

    expect(fetch).toHaveBeenCalledOnce();
  });

  it('dedupes concurrent in-flight requests', async () => {
    let loadNativeReplacements = await importLoader();
    let fetch = vi.fn().mockResolvedValue(Response.json(SAMPLE));

    await Promise.all([loadNativeReplacements(fetch), loadNativeReplacements(fetch)]);

    expect(fetch).toHaveBeenCalledOnce();
  });

  it('resolves to an empty map and warns on a network error', async () => {
    let loadNativeReplacements = await importLoader();
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    let fetch = vi.fn().mockRejectedValue(new Error('offline'));

    await expect(loadNativeReplacements(fetch)).resolves.toEqual({});
  });

  it('resolves to an empty map on a non-2xx response', async () => {
    let loadNativeReplacements = await importLoader();
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    let fetch = vi.fn().mockResolvedValue(Response.json({}, { status: 500 }));

    await expect(loadNativeReplacements(fetch)).resolves.toEqual({});
  });

  it('retries after a failed fetch', async () => {
    let loadNativeReplacements = await importLoader();
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    let fetch = vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce(Response.json(SAMPLE));

    await expect(loadNativeReplacements(fetch)).resolves.toEqual({});
    await expect(loadNativeReplacements(fetch)).resolves.toEqual(SAMPLE);
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('gives up after the timeout but warms the cache for the next call', async () => {
    vi.useFakeTimers();
    let loadNativeReplacements = await importLoader();

    let { promise, resolve } = Promise.withResolvers<Response>();
    let fetch = vi.fn().mockReturnValue(promise);

    let first = loadNativeReplacements(fetch);
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    await expect(first).resolves.toEqual({});

    resolve(Response.json(SAMPLE));

    await expect(loadNativeReplacements(fetch)).resolves.toEqual(SAMPLE);
    expect(fetch).toHaveBeenCalledOnce();
  });
});
