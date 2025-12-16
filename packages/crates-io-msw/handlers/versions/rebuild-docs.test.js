import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 for unknown crates', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  await db.crate.create({ name: 'foo' });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('triggers a rebuild for the crate documentation', async function () {
  let crate = await db.crate.create({ name: 'foo' });
  await db.version.create({ crate, num: '1.0.0' });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/crates/foo/1.0.0/rebuild_docs', { method: 'POST' });
  expect(response.status).toBe(201);
  expect(await response.text()).toBe('');
});
