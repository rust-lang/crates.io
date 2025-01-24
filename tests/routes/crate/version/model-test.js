import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../../helpers/visit-ignoring-abort';

module('Route | crate.version | model() hook', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  module('with explicit version number in the URL', function () {
    test('shows yanked versions', async function (assert) {
      let crate = this.db.crate.create({ name: 'foo' });
      this.db.version.create({ crate, num: '1.0.0' });
      this.db.version.create({ crate, num: '1.2.3', yanked: true });
      this.db.version.create({ crate, num: '2.0.0-beta.1' });

      await visit('/crates/foo/1.2.3');
      assert.strictEqual(currentURL(), `/crates/foo/1.2.3`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('v1.2.3');
      assert.dom('[data-test-yanked]').exists();
      assert.dom('[data-test-docs]').exists();
      assert.dom('[data-test-install]').doesNotExist();
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('shows error page for unknown versions', async function (assert) {
      let crate = this.db.crate.create({ name: 'foo' });
      this.db.version.create({ crate, num: '1.0.0' });
      this.db.version.create({ crate, num: '1.2.3', yanked: true });
      this.db.version.create({ crate, num: '2.0.0-beta.1' });

      await visit('/crates/foo/2.0.0');
      assert.strictEqual(currentURL(), `/crates/foo/2.0.0`);
      assert.dom('[data-test-404-page]').exists();
      assert.dom('[data-test-title]').hasText('foo: Version 2.0.0 not found');
      assert.dom('[data-test-go-back]').exists();
      assert.dom('[data-test-try-again]').doesNotExist();
    });
  });

  module('without version number in the URL', function () {
    test('defaults to the highest stable version', async function (assert) {
      let crate = this.db.crate.create({ name: 'foo' });
      this.db.version.create({ crate, num: '1.0.0' });
      this.db.version.create({ crate, num: '1.2.3', yanked: true });
      this.db.version.create({ crate, num: '2.0.0-beta.1' });
      this.db.version.create({ crate, num: '2.0.0' });

      await visit('/crates/foo');
      assert.strictEqual(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('v2.0.0');
      assert.dom('[data-test-yanked]').doesNotExist();
      assert.dom('[data-test-docs]').exists();
      assert.dom('[data-test-install]').exists();
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('defaults to the highest stable version, even if there are higher prereleases', async function (assert) {
      let crate = this.db.crate.create({ name: 'foo' });
      this.db.version.create({ crate, num: '1.0.0' });
      this.db.version.create({ crate, num: '1.2.3', yanked: true });
      this.db.version.create({ crate, num: '2.0.0-beta.1' });

      await visit('/crates/foo');
      assert.strictEqual(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('v1.0.0');
      assert.dom('[data-test-yanked]').doesNotExist();
      assert.dom('[data-test-docs]').exists();
      assert.dom('[data-test-install]').exists();
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('defaults to the highest not-yanked version', async function (assert) {
      let crate = this.db.crate.create({ name: 'foo' });
      this.db.version.create({ crate, num: '1.0.0', yanked: true });
      this.db.version.create({ crate, num: '1.2.3', yanked: true });
      this.db.version.create({ crate, num: '2.0.0-beta.1' });
      this.db.version.create({ crate, num: '2.0.0-beta.2' });
      this.db.version.create({ crate, num: '2.0.0', yanked: true });

      await visit('/crates/foo');
      assert.strictEqual(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('v2.0.0-beta.2');
      assert.dom('[data-test-yanked]').doesNotExist();
      assert.dom('[data-test-docs]').exists();
      assert.dom('[data-test-install]').exists();
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('if there are only yanked versions, it defaults to the latest version', async function (assert) {
      let crate = this.db.crate.create({ name: 'foo' });
      this.db.version.create({ crate, num: '1.0.0', yanked: true });
      this.db.version.create({ crate, num: '1.2.3', yanked: true });
      this.db.version.create({ crate, num: '2.0.0-beta.1', yanked: true });

      await visit('/crates/foo');
      assert.strictEqual(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('v2.0.0-beta.1');
      assert.dom('[data-test-yanked]').exists();
      assert.dom('[data-test-docs]').exists();
      assert.dom('[data-test-install]').doesNotExist();
      assert.dom('[data-test-notification-message]').doesNotExist();
    });
  });
});
