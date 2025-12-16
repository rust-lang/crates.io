import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('updates the user with a new email address', async function () {
  let user = await db.user.create({ email: 'old@email.com' });
  await db.mswSession.create({ user });

  let body = JSON.stringify({ user: { email: 'new@email.com' } });
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "ok": true,
    }
  `);

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.email).toBe('new@email.com');
  expect(user.emailVerified).toBe(false);
  expect(user.emailVerificationToken).toBe('secret123');
});

test('updates the `publish_notifications` settings', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });
  expect(user.publishNotifications).toBe(true);

  let body = JSON.stringify({ user: { publish_notifications: false } });
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "ok": true,
    }
  `);

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.publishNotifications).toBe(false);
});

test('returns 403 when not logged in', async function () {
  let user = await db.user.create({ email: 'old@email.com' });

  let body = JSON.stringify({ user: { email: 'new@email.com' } });
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
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

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.email).toBe('old@email.com');
});

test('returns 400 when requesting the wrong user id', async function () {
  let user = await db.user.create({ email: 'old@email.com' });
  await db.mswSession.create({ user });

  let body = JSON.stringify({ user: { email: 'new@email.com' } });
  let response = await fetch(`/api/v1/users/wrong-id`, { method: 'PUT', body });
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "current user does not match requested user",
        },
      ],
    }
  `);

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.email).toBe('old@email.com');
});

test('returns 400 when sending an invalid payload', async function () {
  let user = await db.user.create({ email: 'old@email.com' });
  await db.mswSession.create({ user });

  let body = JSON.stringify({});
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "invalid json request",
        },
      ],
    }
  `);

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.email).toBe('old@email.com');
});

test('returns 400 when sending an empty email address', async function () {
  let user = await db.user.create({ email: 'old@email.com' });
  await db.mswSession.create({ user });

  let body = JSON.stringify({ user: { email: '' } });
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "empty email rejected",
        },
      ],
    }
  `);

  user = db.user.findFirst(q => q.where({ id: user.id }));
  expect(user.email).toBe('old@email.com');
});
