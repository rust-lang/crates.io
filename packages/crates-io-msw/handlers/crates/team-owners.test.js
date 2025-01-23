import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/owner_team');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('empty case', async function () {
  db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/owner_team');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    teams: [],
  });
});

test('returns the list of teams that own the specified crate', async function () {
  let team = db.team.create({ name: 'maintainers' });
  let crate = db.crate.create({ name: 'rand' });
  db.crateOwnership.create({ crate, team });

  let response = await fetch('/api/v1/crates/rand/owner_team');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    teams: [
      {
        id: 1,
        avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
        kind: 'team',
        login: 'github:rust-lang:maintainers',
        name: 'maintainers',
        url: 'https://github.com/rust-lang',
      },
    ],
  });
});
