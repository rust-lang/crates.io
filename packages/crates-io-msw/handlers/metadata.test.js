import { assert, expect, test } from 'vitest';

test('returns the deployed SHA1 and read-only status', async function () {
  let response = await fetch('/api/v1/site_metadata');
  assert.strictEqual(response.status, 200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "commit": "5048d31943118c6d67359bd207d307c854e82f45",
      "deployed_sha": "5048d31943118c6d67359bd207d307c854e82f45",
      "read_only": false,
    }
  `);
});
