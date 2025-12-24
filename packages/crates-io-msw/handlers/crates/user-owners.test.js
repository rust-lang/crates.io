import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/owner_user');
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

test('empty case', async function () {
  await db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/owner_user');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "users": [],
    }
  `);
});

test('returns the list of users that own the specified crate', async function () {
  let user = await db.user.create({ name: 'John Doe' });
  let crate = await db.crate.create({ name: 'rand' });
  await db.crateOwnership.create({ crate, user });

  let response = await fetch('/api/v1/crates/rand/owner_user');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "users": [
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "id": 1,
          "kind": "user",
          "login": "john-doe",
          "name": "John Doe",
          "url": "https://github.com/john-doe",
        },
      ],
    }
  `);
});
