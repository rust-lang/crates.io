import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns `ok`', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ ok: true });
});

test('returns 403 when not logged in', async function () {
  let user = await db.user.create();

  let response = await fetch(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({ errors: [{ detail: 'must be logged in to perform that action' }] });
});

test('returns 400 when requesting the wrong user id', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/wrong-id/resend`, { method: 'PUT' });
  expect(response.status).toBe(400);
  expect(await response.json()).toEqual({ errors: [{ detail: 'current user does not match requested user' }] });
});
