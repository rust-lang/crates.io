import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';
import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest';

import { buildPlaygroundLink, loadPlaygroundCrates } from './playground';

describe('buildPlaygroundLink', () => {
  it('serde', () => {
    expect(buildPlaygroundLink('serde')).toMatchInlineSnapshot(
      `"https://play.rust-lang.org/?edition=2021&code=use%20serde%3B%0A%0Afn%20main()%20%7B%0A%20%20%20%20%2F%2F%20try%20using%20the%20%60serde%60%20crate%20here%0A%7D"`,
    );
  });

  it('aho_corasick', () => {
    expect(buildPlaygroundLink('aho_corasick')).toMatchInlineSnapshot(
      `"https://play.rust-lang.org/?edition=2021&code=use%20aho_corasick%3B%0A%0Afn%20main()%20%7B%0A%20%20%20%20%2F%2F%20try%20using%20the%20%60aho_corasick%60%20crate%20here%0A%7D"`,
    );
  });

  it('regex', () => {
    expect(buildPlaygroundLink('regex')).toMatchInlineSnapshot(
      `"https://play.rust-lang.org/?edition=2021&code=use%20regex%3B%0A%0Afn%20main()%20%7B%0A%20%20%20%20%2F%2F%20try%20using%20the%20%60regex%60%20crate%20here%0A%7D"`,
    );
  });
});

describe('loadPlaygroundCrates', () => {
  let server = setupServer();
  beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());

  it('crates are available if the request succeeds', async () => {
    let crates = [
      { name: 'addr2line', version: '0.14.1', id: 'addr2line' },
      { name: 'adler', version: '0.2.3', id: 'adler' },
      { name: 'adler32', version: '1.2.0', id: 'adler32' },
      { name: 'ahash', version: '0.4.7', id: 'ahash' },
      { name: 'aho-corasick', version: '0.7.15', id: 'aho_corasick' },
      { name: 'ansi_term', version: '0.12.1', id: 'ansi_term' },
      { name: 'ansi_term', version: '0.11.0', id: 'ansi_term_0_11_0' },
    ];

    server.use(http.get('https://play.rust-lang.org/meta/crates', () => HttpResponse.json({ crates })));

    let result = await loadPlaygroundCrates(fetch);
    expect(result).toEqual(crates);
  });

  it('loadPlaygroundCrates fails on HTTP error', async () => {
    server.use(http.get('https://play.rust-lang.org/meta/crates', () => HttpResponse.json({}, { status: 500 })));

    await expect(loadPlaygroundCrates(fetch)).rejects.toThrow('Failed to load Rust Playground crates');
  });
});
