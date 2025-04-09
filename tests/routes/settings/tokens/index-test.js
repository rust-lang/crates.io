import { click, currentURL, fillIn, findAll } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../../helpers/visit-ignoring-abort';

module('/settings/tokens', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    context.authenticateAs(user);

    return { user };
  }

  test('reloads all tokens from the server', async function (assert) {
    let { user } = prepare(this);

    this.db.apiToken.create({ user, name: 'token-1' });

    await visit('/settings/tokens/new');
    assert.strictEqual(currentURL(), '/settings/tokens/new');

    await fillIn('[data-test-name]', 'token-2');
    await click('[data-test-scope="publish-update"]');
    await click('[data-test-generate]');

    assert.strictEqual(currentURL(), '/settings/tokens');
    assert.dom('[data-test-api-token]').exists({ count: 2 });
    let tokens = findAll('[data-test-api-token]');
    assert.dom('[data-test-name]', tokens[0]).hasText('token-2');
    assert.dom('[data-test-token]', tokens[0]).exists();
    assert.dom('[data-test-name]', tokens[1]).hasText('token-1');
    assert.dom('[data-test-token]', tokens[1]).doesNotExist();
  });
});
