import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('revokes an API token', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let token = await db.apiToken.create({ user });

  let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`{}`);

  let tokens = db.apiToken.findMany(null);
  expect(tokens.length).toBe(0);
});

test('returns an error if unauthenticated', async function () {
  let user = await db.user.create();
  let token = await db.apiToken.create({ user });

  let response = await fetch(`/api/v1/me/tokens/${token.id}`, { method: 'DELETE' });
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "must be logged in to perform that action",
        },
      ],
    }
  `);
});
