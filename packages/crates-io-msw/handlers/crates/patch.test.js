import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ crate: { trustpub_only: true } }),
  });
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
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ crate: { trustpub_only: true } }),
  });
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "crate \`foo\` does not exist",
        },
      ],
    }
  `);
});

test('updates trustpub_only flag', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo', trustpubOnly: false });
  expect(crate.trustpubOnly).toBe(false);

  await db.version.create({ crate, num: '1.0.0' });
  await db.crateOwnership.create({ crate, user });

  let response = await fetch('/api/v1/crates/foo', {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ crate: { trustpub_only: true } }),
  });
  expect(response.status).toBe(200);

  let json = await response.json();
  expect(json.crate.trustpub_only).toBe(true);

  let updatedCrate = db.crate.findFirst(q => q.where({ name: 'foo' }));
  expect(updatedCrate.trustpubOnly).toBe(true);
});
