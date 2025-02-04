import { assert, test } from 'vitest';

test('returns 200 OK and an empty object', async function () {
  let response = await fetch('https://docs.rs/crate/foo/0.0.0/status.json');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {});
});
