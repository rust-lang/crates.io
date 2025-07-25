import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns an error for unauthenticated requests', async function () {
  let response = await fetch('/api/v1/users/1/emails/1/notifications', { method: 'PUT' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns an error for requests to a different user', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/users/512/emails/1/notifications', { method: 'PUT' });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'current user does not match requested user' }],
  });
});

test('returns an error for non-existent email', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/${user.id}/emails/999/notifications`, { method: 'PUT' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Email not found.' }],
  });
});

test('successfully enables notifications', async function () {
  let email = db.email.create({ send_notifications: false });
  let user = db.user.create({ emails: [email] });

  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/${user.id}/emails/${email.id}/notifications`, { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  let updatedEmail = await response.json();
  assert.strictEqual(updatedEmail.send_notifications, true);
  assert.strictEqual(updatedEmail.email, 'foo@crates.io');

  // Verify the change was persisted
  let emailFromDb = db.email.findFirst({ where: { id: { equals: email.id } } });
  assert.strictEqual(emailFromDb.send_notifications, true);
});
