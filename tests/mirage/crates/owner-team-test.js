import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:id/owner_team', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/owner_team');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('empty case', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/owner_team');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      teams: [],
    });
  });

  test('returns the list of teams that own the specified crate', async function (assert) {
    let team = this.server.create('team', { name: 'maintainers' });
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('crate-ownership', { crate, team });

    let response = await fetch('/api/v1/crates/rand/owner_team');
    assert.equal(response.status, 200);
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
});
