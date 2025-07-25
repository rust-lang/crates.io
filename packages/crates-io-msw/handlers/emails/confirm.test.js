import { assert, test } from 'vitest';

import { db } from '../../index.js';
import { serializeEmail } from '../../serializers/email.js';

test('returns `ok: true` for a known token (unauthenticated)', async function () {
  let email = db.email.create({ token: 'foo' });
  let user = db.user.create({ emails: [email] });
  assert.strictEqual(email.verified, false);

  let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true, email: serializeEmail({ ...email, verified: true }) });

  email = db.email.findFirst({ where: { id: user.emails[0].id } });
  assert.strictEqual(email.verified, true);
});

test('returns `ok: true` for a known token (authenticated)', async function () {
  let email = db.email.create({ token: 'foo' });
  let user = db.user.create({ emails: [email] });
  assert.strictEqual(email.verified, false);

  db.mswSession.create({ user });

  let response = await fetch('/api/v1/confirm/foo', { method: 'PUT' });
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true, email: serializeEmail({ ...email, verified: true }) });

  email = db.email.findFirst({ where: { id: user.emails[0].id } });
  assert.strictEqual(email.verified, true);
});

test('returns an error for unknown tokens', async function () {
  let response = await fetch('/api/v1/confirm/unknown', { method: 'PUT' });
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Email belonging to token not found.' }],
  });
});
