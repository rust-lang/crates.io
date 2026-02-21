import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | support', function (hooks) {
  setupApplicationTest(hooks);

  async function prepare(context) {
    let user = await context.db.user.create({});
    await context.authenticateAs(user);
  }

  test('footer should always point to /support without query parameters', async function (assert) {
    // Link to /support when not authenticated is fine
    await visit('/');
    assert.dom('footer [data-test-support-link]').hasAttribute('href', '/support');

    // But visiting /support requires authentication
    await prepare(this);
    await visit('/support?inquire=crate-violation&crate=foo');
    assert.dom('footer [data-test-support-link]').hasAttribute('href', '/support');

    await click('header [href="/"]');
    assert.dom('footer [data-test-support-link]').hasAttribute('href', '/support');
  });

  test('should not retain query params when exiting and then returning', async function (assert) {
    await prepare(this);

    await visit('/support?inquire=crate-violation');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');
    assert
      .dom('[data-test-id="support-main-content"] section')
      .exists({ count: 1 })
      .hasAttribute('data-test-id', 'crate-violation-section');

    // back to index
    await click('header [href="/"]');
    assert.strictEqual(currentURL(), '/');
    assert.dom('footer [data-test-support-link]').hasAttribute('href', '/support');

    // goto support
    await click('footer [data-test-support-link]');
    assert.strictEqual(currentURL(), '/support');
    assert
      .dom('[data-test-id="support-main-content"] section')
      .exists({ count: 1 })
      .hasAttribute('data-test-id', 'inquire-list-section');
    await click('[data-test-id="link-crate-violation"]');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');
  });
});
