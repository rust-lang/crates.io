import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/owner_user');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('empty case', async function () {
  db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/owner_user');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    users: [],
  });
});

test('returns the list of users that own the specified crate', async function () {
  let user = db.user.create({ name: 'John Doe' });
  let crate = db.crate.create({ name: 'rand' });
  db.crateOwnership.create({ crate, user });

  let response = await fetch('/api/v1/crates/rand/owner_user');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    users: [
      {
        id: 1,
        avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
        kind: 'user',
        login: 'john-doe',
        name: 'John Doe',
        url: 'https://github.com/john-doe',
      },
    ],
  });
});
