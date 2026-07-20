import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown users', async function () {
  let response = await fetch('/api/v1/users/42/stats');
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

test('returns 0 total downloads for a user with no owned crates', async function () {
  let user = await db.user.create({ name: 'John Doe' });

  let response = await fetch(`/api/v1/users/${user.id}/stats`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "total_downloads": 0,
    }
  `);
});

test('returns total downloads across all owned crates', async function () {
  let user = await db.user.create({ name: 'John Doe' });

  let crate1 = await db.crate.create({ name: 'rand', downloads: 1000 });
  let crate2 = await db.crate.create({ name: 'serde', downloads: 2500 });
  await db.crateOwnership.create({ crate: crate1, user });
  await db.crateOwnership.create({ crate: crate2, user });

  let response = await fetch(`/api/v1/users/${user.id}/stats`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "total_downloads": 3500,
    }
  `);
});

test('does not include downloads from crates owned by other users', async function () {
  let user1 = await db.user.create({ name: 'John Doe' });
  let user2 = await db.user.create({ name: 'Jane Doe' });

  let crate1 = await db.crate.create({ name: 'rand', downloads: 1000 });
  let crate2 = await db.crate.create({ name: 'serde', downloads: 2500 });
  await db.crateOwnership.create({ crate: crate1, user: user1 });
  await db.crateOwnership.create({ crate: crate2, user: user2 });

  let response = await fetch(`/api/v1/users/${user1.id}/stats`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "total_downloads": 1000,
    }
  `);
});
