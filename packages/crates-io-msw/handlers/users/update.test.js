import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('updates the `publish_notifications` settings', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });
  assert.strictEqual(user.publishNotifications, true);

  let body = JSON.stringify({ user: { publish_notifications: false } });
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  user = db.user.findFirst({ where: { id: user.id } });
  assert.strictEqual(user.publishNotifications, false);
});

test('returns 403 when not logged in', async function () {
  let user = db.user.create({ emails: [db.email.create()] });
  assert.strictEqual(user.publishNotifications, true);

  let body = JSON.stringify({ user: { publish_notifications: false } });
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'must be logged in to perform that action' }] });

  user = db.user.findFirst({ where: { id: user.id } });
  assert.strictEqual(user.publishNotifications, true);
});

test('returns 400 when requesting the wrong user id', async function () {
  let user = db.user.create({ emails: [db.email.create()] });
  assert.strictEqual(user.publishNotifications, true);
  db.mswSession.create({ user });

  let body = JSON.stringify({ user: { publish_notifications: false } });
  let response = await fetch(`/api/v1/users/wrong-id`, { method: 'PUT', body });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'current user does not match requested user' }] });

  user = db.user.findFirst({ where: { id: user.id } });
  assert.strictEqual(user.publishNotifications, true);
});

test('returns 400 when sending an invalid payload', async function () {
  let user = db.user.create({ emails: [db.email.create()] });
  db.mswSession.create({ user });

  let body = JSON.stringify({});
  let response = await fetch(`/api/v1/users/${user.id}`, { method: 'PUT', body });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'invalid json request' }] });
});
