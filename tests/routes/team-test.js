import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | team', function (hooks) {
  setupApplicationTest(hooks);

  test("shows an error message if the user can't be found", async function (assert) {
    await visit('/teams/foo');
    assert.strictEqual(currentURL(), '/teams/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Team not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('server error causes the error page to be shown', async function (assert) {
    this.server.get('/api/v1/teams/:id', {}, 500);

    await visit('/teams/foo');
    assert.strictEqual(currentURL(), '/teams/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load team data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });
});
