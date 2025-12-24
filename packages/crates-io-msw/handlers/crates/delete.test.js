import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
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

  let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
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

test('deletes crates', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let crate = await db.crate.create({ name: 'foo' });
  await db.crateOwnership.create({ crate, user });

  let response = await fetch('/api/v1/crates/foo', { method: 'DELETE' });
  expect(response.status).toBe(204);
  expect(await response.text()).toMatchInlineSnapshot(`""`);

  expect(db.crate.findFirst(q => q.where({ name: 'foo' }))).toBe(undefined);
});
