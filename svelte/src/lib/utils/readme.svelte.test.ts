import { http, HttpResponse } from 'msw';
import { beforeEach, describe, expect } from 'vitest';

import { test } from '../../test/msw';
import { loadReadme } from './readme';
import { resetSiteMetadataCache } from './site-metadata';

const METADATA = {
  commit: 'x',
  deployed_sha: 'x',
  read_only: false,
  cdn_base: 'https://static.crates.io',
};

// `loadReadme` reads site metadata from a module-scoped cache, so clear it
// before each test to keep them isolated.
beforeEach(() => {
  resetSiteMetadataCache();
});

describe('loadReadme', () => {
  test('fetches the README from the reported CDN host', async ({ worker }) => {
    worker.use(
      http.get('/api/v1/site_metadata', () => HttpResponse.json({ ...METADATA, cdn_base: 'https://cdn.example.com' })),
      http.get('https://cdn.example.com/readmes/rand/rand-1.0.0.html', () => HttpResponse.html('<p>hello</p>')),
    );

    expect(await loadReadme(fetch, 'rand', '1.0.0')).toBe('<p>hello</p>');
  });

  test('returns null when there is no README', async ({ worker }) => {
    worker.use(
      http.get('/api/v1/site_metadata', () => HttpResponse.json(METADATA)),
      http.get('https://static.crates.io/readmes/rand/rand-1.0.0.html', () => HttpResponse.html('', { status: 403 })),
    );

    expect(await loadReadme(fetch, 'rand', '1.0.0')).toBeNull();
  });

  test('encodes build metadata in the version', async ({ worker }) => {
    worker.use(
      http.get('/api/v1/site_metadata', () => HttpResponse.json(METADATA)),
      http.get('https://static.crates.io/readmes/rand/rand-1.0.0%2Bfoo.html', () => HttpResponse.html('<p>build</p>')),
    );

    expect(await loadReadme(fetch, 'rand', '1.0.0+foo')).toBe('<p>build</p>');
  });

  test('throws when the request fails unexpectedly', async ({ worker }) => {
    worker.use(
      http.get('/api/v1/site_metadata', () => HttpResponse.json(METADATA)),
      http.get('https://static.crates.io/readmes/rand/rand-1.0.0.html', () => HttpResponse.html('', { status: 500 })),
    );

    await expect(loadReadme(fetch, 'rand', '1.0.0')).rejects.toThrow('Failed to load README');
  });
});
