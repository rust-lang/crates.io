import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/teams/:id', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown teams', async function (assert) {
    let response = await fetch('/api/v1/teams/foo');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns a team object for known teams', async function (assert) {
    let team = this.server.create('team', { name: 'maintainers' });

    let response = await fetch(`/api/v1/teams/${team.login}`);
    assert.equal(response.status, 200);
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
});
