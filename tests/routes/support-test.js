import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import Service from '@ember/service';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | support', function (hooks) {
  setupApplicationTest(hooks);

  test('should not retain query params when exiting and then returning', async function (assert) {
    await visit('/support?inquire=crate-violation');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');
    assert
      .dom('[data-test-id="support-main-content"] section')
      .exists({ count: 1 })
      .hasAttribute('data-test-id', 'crate-violation-section');

    // back to index
    await click('header [href="/"]');
    assert.strictEqual(currentURL(), '/');
    assert.dom('footer [href="/support"]').exists();

    // goto support
    await click('footer [href="/support"]');
    assert.strictEqual(currentURL(), '/support');
    assert
      .dom('[data-test-id="support-main-content"] section')
      .exists({ count: 1 })
      .hasAttribute('data-test-id', 'inquire-list-section');
    await click('[data-test-id="link-crate-violation"]');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');
  });

  test('LinkTo support must overwite query', async function (assert) {
    // query params of LinkTo support's in footer will not be cleared
    class MockService extends Service {
      paramsFor() {
        return {};
      }
    }
    this.owner.register('service:pristine-query', MockService);

    await visit('/support?inquire=crate-violation');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');
    assert
      .dom('[data-test-id="support-main-content"] section')
      .exists({ count: 1 })
      .hasAttribute('data-test-id', 'crate-violation-section');
    // without overwriting, link in footer will contain the query params in support route
    assert.dom('footer [href^="/support"]').doesNotMatchSelector('[href="/support"]');
    assert.dom('footer [href^="/support"]').hasAttribute('href', '/support?inquire=crate-violation');

    // back to index
    await click('header [href="/"]');
    assert.strictEqual(currentURL(), '/');
    assert.dom('footer [href^="/support"]').hasAttribute('href', '/support');
  });

  test('must reset query when existing', async function (assert) {
    const route = this.owner.lookup('route:support');
    const originResetController = route.resetController;
    // query params of LinkTo support's in footer will not be cleared
    class MockService extends Service {
      paramsFor() {
        return {};
      }
    }
    this.owner.register('service:pristine-query', MockService);
    // exiting support will not reset query
    route.resetController = () => {};

    await visit('/support?inquire=crate-violation');
    assert.strictEqual(currentURL(), '/support?inquire=crate-violation');
    assert
      .dom('[data-test-id="support-main-content"] section')
      .exists({ count: 1 })
      .hasAttribute('data-test-id', 'crate-violation-section');

    // back to index
    await click('header [href="/"]');
    assert.strictEqual(currentURL(), '/');
    // without resetController to reset, link in footer will contain the query params in other route
    assert.dom('footer [href^="/support"]').doesNotMatchSelector('[href="/support"]');
    assert.dom('footer [href^="/support"]').hasAttribute('href', '/support?inquire=crate-violation');

    route.resetController = originResetController;
  });
});
