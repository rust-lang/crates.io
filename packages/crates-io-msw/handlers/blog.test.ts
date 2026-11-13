import { expect, test } from 'vitest';

test('returns 200 OK and an Atom feed', async function () {
  let response = await fetch('https://blog.rust-lang.org/tags/crates-io/feed.xml');
  expect(response.status).toBe(200);
  expect(response.headers.get('content-type')).toBe('application/xml');

  let feed = await response.text();
  expect(feed).toContain('<feed xmlns="http://www.w3.org/2005/Atom"');
  expect(feed.match(/<link rel="alternate" href="[^"]+"/g)).toMatchInlineSnapshot(`
    [
      "<link rel="alternate" href="https://blog.rust-lang.org/2025/07/11/crates-io-development-update/"",
      "<link rel="alternate" href="https://blog.rust-lang.org/inside-rust/2025/06/20/crates-io-new-frontend/"",
      "<link rel="alternate" href="https://blog.rust-lang.org/2025/05/24/crates-io-malicious-crates/"",
      "<link rel="alternate" href="https://blog.rust-lang.org/2025/03/18/crates-io-usage-policy/"",
    ]
  `);
});
