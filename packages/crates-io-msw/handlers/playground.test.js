import { expect, test } from 'vitest';

test('returns 200 OK and an empty array', async function () {
  let response = await fetch('https://play.rust-lang.org/meta/crates');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`[]`);
});
