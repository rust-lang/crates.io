import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';
import fetch from 'fetch';

module('Mirage | Teams', function(hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/teams/:id', function() {
    test('returns 404 for unknown teams', async function(assert) {
      let response = await fetch('/api/v1/teams/foo');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('returns a team object for known teams', async function(assert) {
      let team = this.server.create('team', { name: 'maintainers' });

      let response = await fetch(`/api/v1/teams/${team.login}`);
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        team: {
          id: '1',
          avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
          login: 'github:rust-lang:maintainers',
          name: 'maintainers',
          url: 'https://github.com/rust-lang',
        },
      });
    });
  });
});
