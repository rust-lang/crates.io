import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns an error for unauthenticated requests', async function () {
  let response = await fetch('/api/v1/users/1/emails', { method: 'POST' });
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns an error for requests to a different user', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/users/512/emails', { method: 'POST' });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'current user does not match requested user' }],
  });
});

test('returns email for valid, authenticated request', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/users/${user.id}/emails`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email: 'test@example.com' }),
  });
  assert.strictEqual(response.status, 200);
  let email = await response.json();
  assert.strictEqual(email.email, 'test@example.com');
  assert.strictEqual(email.verified, false);
  assert.strictEqual(email.verification_email_sent, true);
  assert.strictEqual(email.primary, false);
});
