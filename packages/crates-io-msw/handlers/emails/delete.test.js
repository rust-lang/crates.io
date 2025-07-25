import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns an error for unauthenticated requests', async function () {
  let response = await fetch('/api/v1/users/1/emails/1', { method: 'DELETE' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns an error for requests to a different user', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/users/512/emails/1', { method: 'DELETE' });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'current user does not match requested user' }],
  });
});

test('returns an error for non-existent email', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/${user.id}/emails/999`, { method: 'DELETE' });
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Email not found.' }],
  });
});

test('prevents deletion of notification email', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let email = db.email.create({ user_id: user.id, email: 'test@example.com', send_notifications: true });

  let response = await fetch(`/api/v1/users/${user.id}/emails/${email.id}`, { method: 'DELETE' });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Cannot delete an email that has notifications enabled.' }],
  });
});

test('successfully deletes alternate email', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let email1 = db.email.create({ user_id: user.id, email: 'test1@example.com', send_notifications: true });
  let email2 = db.email.create({ user_id: user.id, email: 'test2@example.com' });

  let response = await fetch(`/api/v1/users/${user.id}/emails/${email2.id}`, { method: 'DELETE' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true });

  // Check that email2 was deleted
  let deletedEmail = db.email.findFirst({ where: { id: { equals: email2.id } } });
  assert.strictEqual(deletedEmail, null);

  // Check that email1 still exists
  let remainingEmail = db.email.findFirst({ where: { id: { equals: email1.id } } });
  assert.strictEqual(remainingEmail.email, 'test1@example.com');
});
