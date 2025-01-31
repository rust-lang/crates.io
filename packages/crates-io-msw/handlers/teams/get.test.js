import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown teams', async function () {
  let response = await fetch('/api/v1/teams/foo');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns a team object for known teams', async function () {
  let team = db.team.create({ name: 'maintainers' });

  let response = await fetch(`/api/v1/teams/${team.login}`);
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    team: {
      id: 1,
      avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
      login: 'github:rust-lang:maintainers',
      name: 'maintainers',
      url: 'https://github.com/rust-lang',
    },
  });
});
