import { assert, test } from 'vitest';

test('returns 200 OK and an empty array', async function () {
  let response = await fetch('https://play.rust-lang.org/meta/crates');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), []);
});
