import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown users', async function () {
  let response = await fetch('/api/v1/users/foo');
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

test('returns a user object for known users', async function () {
  let user = await db.user.create({ name: 'John Doe' });

  let response = await fetch(`/api/v1/users/${user.login}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "id": 1,
        "login": "john-doe",
        "name": "John Doe",
        "url": "https://github.com/john-doe",
      },
    }
  `);
});
