import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
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

  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Not Found",
        },
      ],
    }
  `);
});

test('returns 404 for unknown versions', async function () {
  await db.crate.create({ name: 'foo' });

  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Not Found",
        },
      ],
    }
  `);
});

test('unyanks the version', async function () {
  let crate = await db.crate.create({ name: 'foo' });
  let version = await db.version.create({ crate, num: '1.0.0', yanked: true, yank_message: 'some reason' });
  expect(version.yanked).toBe(true);
  expect(version.yank_message).toBe('some reason');

  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/unyank', { method: 'PUT' });
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "ok": true,
    }
  `);

  version = db.version.findFirst(q => q.where({ id: version.id }));
  expect(version.yanked).toBe(false);
  expect(version.yank_message).toBe(null);
});
