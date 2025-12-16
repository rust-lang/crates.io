import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns `ok: true` for a known token (unauthenticated)', async function () {
  let user = await db.user.create({ emailVerificationToken: 'foo' });
  expect(user.emailVerified).toBe(false);

  let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ ok: true });

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.emailVerified).toBe(true);
});

test('returns `ok: true` for a known token (authenticated)', async function () {
  let user = await db.user.create({ emailVerificationToken: 'foo' });
  expect(user.emailVerified).toBe(false);

  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ ok: true });

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.emailVerified).toBe(true);
});

test('returns an error for unknown tokens', async function () {
  let response = await fetch('/api/v1/confirm/unknown', { method: 'PUT' });
  expect(response.status).toBe(400);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'Email belonging to token not found.' }],
  });
});
