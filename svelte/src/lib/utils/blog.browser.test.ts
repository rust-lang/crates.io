import { http, HttpResponse } from 'msw';
import { describe, expect } from 'vitest';

import { test } from '../../test/msw';
import { loadBlogFeed } from './blog';

const FEED_URL = 'https://blog.rust-lang.org/tags/crates-io/feed.xml';

describe('loadBlogFeed', () => {
  test('returns the parsed feed document', async ({ worker }) => {
    let xml = '<feed xmlns="http://www.w3.org/2005/Atom"><title>Rust Blog</title></feed>';
    worker.use(http.get(FEED_URL, () => HttpResponse.xml(xml)));

    let feed = await loadBlogFeed(fetch);
    expect(feed.documentElement.localName).toBe('feed');
    expect(feed.documentElement.textContent).toBe('Rust Blog');
  });

  test('fails on HTTP error', async ({ worker }) => {
    worker.use(http.get(FEED_URL, () => HttpResponse.text('', { status: 500 })));

    await expect(loadBlogFeed(fetch)).rejects.toThrow('Failed to load blog feed');
  });

  test('fails on invalid XML', async ({ worker }) => {
    worker.use(http.get(FEED_URL, () => HttpResponse.text('<feed>')));

    await expect(loadBlogFeed(fetch)).rejects.toThrow('Failed to parse blog feed');
  });
});
