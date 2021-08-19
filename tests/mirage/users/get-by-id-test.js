import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/users/:id', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown users', async function (assert) {
    let response = await fetch('/api/v1/users/foo');
    assert.equal(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns a user object for known users', async function (assert) {
    let user = this.server.create('user', { name: 'John Doe' });

    let response = await fetch(`/api/v1/users/${user.login}`);
    assert.equal(response.status, 200);
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
});
