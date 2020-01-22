import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../helpers/setup-mirage';
import fetch from 'fetch';

module('Mirage | Users', function(hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/users/:id', function() {
    test('returns 404 for unknown users', async function(assert) {
      let response = await fetch('/api/v1/users/foo');
      assert.equal(response.status, 404);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { errors: [{ detail: 'Not Found' }] });
    });

    test('returns a user object for known users', async function(assert) {
      let user = this.server.create('user', { name: 'John Doe' });

      let response = await fetch(`/api/v1/users/${user.login}`);
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        user: {
          id: '1',
          avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
          login: 'john-doe',
          name: 'John Doe',
          url: 'https://github.com/john-doe',
        },
      });
    });
  });
});
