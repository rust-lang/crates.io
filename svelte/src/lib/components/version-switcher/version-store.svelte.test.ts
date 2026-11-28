import { db, handlers } from '@crates-io/msw';
import { http, HttpResponse } from 'msw';
import { afterEach, describe, expect } from 'vitest';

import { test } from '../../../test/msw';
import { VersionStore } from './version-store.svelte';

afterEach(() => db.reset());

async function createCrate(numVersions: number) {
  let crate = await db.crate.create({ name: 'foo' });
  for (let i = 0; i < numVersions; i++) {
    await db.version.create({ crate, num: `1.0.${i}` });
  }
}

function expectedNums(from: number, to: number) {
  let nums = [];
  for (let i = from; i >= to; i--) {
    nums.push(`1.0.${i}`);
  }
  return nums;
}

describe('VersionStore', () => {
  test('loads 25 versions first and then 50 per page in semver order', async ({ worker }) => {
    worker.use(...handlers);
    await createCrate(100);

    let store = new VersionStore('foo');
    expect(store.versions).toEqual([]);
    expect(store.isComplete).toBe(false);

    await store.loadMore();
    expect(store.versions.map(v => v.num)).toEqual(expectedNums(99, 75));
    expect(store.isComplete).toBe(false);

    await store.loadMore();
    expect(store.versions.map(v => v.num)).toEqual(expectedNums(99, 25));
    expect(store.isComplete).toBe(false);

    await store.loadMore();
    expect(store.versions.map(v => v.num)).toEqual(expectedNums(99, 0));
    expect(store.isComplete).toBe(true);
  });

  test('is complete after the first page for crates with few versions', async ({ worker }) => {
    worker.use(...handlers);
    await createCrate(3);

    let store = new VersionStore('foo');
    await store.loadMore();
    expect(store.versions.map(v => v.num)).toEqual(['1.0.2', '1.0.1', '1.0.0']);
    expect(store.isComplete).toBe(true);
  });

  test('sends a single request for concurrent calls', async ({ worker }) => {
    worker.use(...handlers);
    await createCrate(3);

    let requests: string[] = [];
    worker.events.on('request:start', ({ request }) => requests.push(request.url));

    let store = new VersionStore('foo');
    let first = store.loadMore();
    expect(store.isLoading).toBe(true);
    await Promise.all([first, store.loadMore()]);
    expect(store.isLoading).toBe(false);
    expect(requests).toHaveLength(1);

    await store.loadMore();
    expect(requests).toHaveLength(1);
    expect(store.versions).toHaveLength(3);

    worker.events.removeAllListeners();
  });

  test('keeps the loaded versions and allows retrying after a failed request', async ({ worker }) => {
    worker.use(...handlers);
    await createCrate(60);

    let store = new VersionStore('foo');
    await store.loadMore();

    worker.use(http.get('/api/v1/crates/:name/versions', () => HttpResponse.json({}, { status: 500 }), { once: true }));
    await store.loadMore();
    expect(store.error).toBe(true);
    expect(store.isLoading).toBe(false);
    expect(store.isComplete).toBe(false);
    expect(store.versions).toHaveLength(25);

    await store.loadMore();
    expect(store.error).toBe(false);
    expect(store.isComplete).toBe(true);
    expect(store.versions).toHaveLength(60);
  });
});
