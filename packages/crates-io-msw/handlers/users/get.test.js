import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown users', async function () {
  let response = await fetch('/api/v1/users/foo');
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('returns a user object for known users', async function () {
  let user = await db.user.create({ name: 'John Doe' });

  let response = await fetch(`/api/v1/users/${user.login}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    user: {
      id: 1,
      avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
      login: 'john-doe',
      name: 'John Doe',
      url: 'https://github.com/john-doe',
    },
  });
});
