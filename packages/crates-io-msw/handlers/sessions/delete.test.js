import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 200 when authenticated', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/private/session', { method: 'DELETE' });
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ ok: true });

  expect(db.mswSession.findFirst(null)).toBeFalsy();
});

test('returns 200 when unauthenticated', async function () {
  let response = await fetch('/api/private/session', { method: 'DELETE' });
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ ok: true });

  expect(db.mswSession.findFirst(null)).toBeFalsy();
});
