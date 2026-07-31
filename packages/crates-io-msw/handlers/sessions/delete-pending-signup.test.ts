import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('clears the pending signup', async function () {
  await db.pendingSignup.create({ login: 'ghost' });

  let response = await fetch('/api/private/session/signup', { method: 'DELETE' });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "ok": true,
    }
  `);

  expect(db.pendingSignup.findFirst()).toBeFalsy();
});

test('returns 200 without a pending signup', async function () {
  let response = await fetch('/api/private/session/signup', { method: 'DELETE' });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "ok": true,
    }
  `);

  expect(db.pendingSignup.findFirst()).toBeFalsy();
});
