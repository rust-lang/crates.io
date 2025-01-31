import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown users', async function () {
  let response = await fetch('/api/v1/users/foo');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns a user object for known users', async function () {
  let user = db.user.create({ name: 'John Doe' });

  let response = await fetch(`/api/v1/users/${user.login}`);
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    user: {
      id: 1,
      avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
      login: 'john-doe',
      name: 'John Doe',
      url: 'https://github.com/john-doe',
    },
  });
});
