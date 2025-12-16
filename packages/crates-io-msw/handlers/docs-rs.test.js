import { expect, test } from 'vitest';

test('returns 200 OK and an empty object', async function () {
  let response = await fetch('https://docs.rs/crate/foo/0.0.0/status.json');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`{}`);
});
