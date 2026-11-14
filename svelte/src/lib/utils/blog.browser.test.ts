import { http, HttpResponse } from 'msw';
import { describe, expect } from 'vitest';

import { test } from '../../test/msw';
import { loadBlogFeed, parseBlogPosts } from './blog';

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

describe('parseBlogPosts', () => {
  function feed(...entries: string[]) {
    let xml = `<feed xmlns="http://www.w3.org/2005/Atom">${entries.join('')}</feed>`;
    return new DOMParser().parseFromString(xml, 'application/xml');
  }

  function entry({
    title = 'crates.io: development update',
    url = 'https://blog.rust-lang.org/2025/07/11/crates-io-development-update/',
    published = '2025-07-11T00:00:00+00:00',
  } = {}) {
    return `<entry><title>${title}</title><link rel="alternate" href="${url}"/><published>${published}</published></entry>`;
  }

  test('returns the title, URL and publication date of each entry', () => {
    expect(parseBlogPosts(feed(entry()))).toEqual([
      {
        title: 'crates.io: development update',
        url: 'https://blog.rust-lang.org/2025/07/11/crates-io-development-update/',
        published: new Date('2025-07-11T00:00:00Z'),
        isInsideRust: false,
      },
    ]);
  });

  test('decodes entities in titles', () => {
    let [post] = parseBlogPosts(feed(entry({ title: 'Usage policy &amp; &lt;b&gt;trusted publishing&lt;/b&gt;' })));
    expect(post?.title).toBe('Usage policy & <b>trusted publishing</b>');
  });

  test('marks Inside Rust posts', () => {
    let url = 'https://blog.rust-lang.org/inside-rust/2025/06/20/crates-io-new-frontend/';
    let [post] = parseBlogPosts(feed(entry({ url })));
    expect(post?.isInsideRust).toBe(true);
  });

  test('skips entries linking outside of the Rust blog', () => {
    let posts = parseBlogPosts(
      feed(
        entry({ url: 'javascript:alert(1)' }),
        entry({ url: 'https://example.com/' }),
        entry({ url: 'https://blog.rust-lang.org.example.com/' }),
        entry({ url: 'http://blog.rust-lang.org/2025/01/01/post/' }),
        entry({ url: 'https://blog.rust-lang.org/2025/01/01/post/' }),
      ),
    );
    expect(posts.map(post => post.url)).toEqual(['https://blog.rust-lang.org/2025/01/01/post/']);
  });

  test('skips entries without title or with an invalid publication date', () => {
    let posts = parseBlogPosts(
      feed(entry({ title: '' }), entry({ published: '' }), entry({ published: 'yesterday' }), entry()),
    );
    expect(posts.map(post => post.title)).toEqual(['crates.io: development update']);
  });

  test('sorts posts by publication date, newest first', () => {
    let posts = parseBlogPosts(
      feed(
        entry({ title: 'b', published: '2025-03-18T00:00:00+00:00' }),
        entry({ title: 'c', published: '2025-07-11T00:00:00+00:00' }),
        entry({ title: 'a', published: '2024-12-01T00:00:00+00:00' }),
      ),
    );
    expect(posts.map(post => post.title)).toEqual(['c', 'b', 'a']);
  });

  test('returns an empty list if the feed contains no entries', () => {
    expect(parseBlogPosts(feed())).toEqual([]);
  });
});
