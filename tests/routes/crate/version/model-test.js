import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../../../helpers/visit-ignoring-abort';

module('Route | crate.version | model() hook', function (hooks) {
  setupApplicationTest(hooks);

  module('with explicit version number in the URL', function () {
    test('shows yanked versions', async function (assert) {
      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '1.0.0' });
      this.server.create('version', { crate, num: '1.2.3', yanked: true });
      this.server.create('version', { crate, num: '2.0.0-beta.1' });

      await visit('/crates/foo/1.2.3');
      assert.equal(currentURL(), `/crates/foo/1.2.3`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('1.2.3');
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('shows error page for unknown versions', async function (assert) {
      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '1.0.0' });
      this.server.create('version', { crate, num: '1.2.3', yanked: true });
      this.server.create('version', { crate, num: '2.0.0-beta.1' });

      await visit('/crates/foo/2.0.0');
      assert.equal(currentURL(), `/crates/foo/2.0.0`);
      assert.dom('[data-test-404-page]').exists();
      assert.dom('[data-test-title]').hasText('Version not found');
      assert.dom('[data-test-go-back]').exists();
      assert.dom('[data-test-try-again]').doesNotExist();
    });
  });

  module('without version number in the URL', function () {
    test('defaults to the highest stable version', async function (assert) {
      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '1.0.0' });
      this.server.create('version', { crate, num: '1.2.3', yanked: true });
      this.server.create('version', { crate, num: '2.0.0-beta.1' });
      this.server.create('version', { crate, num: '2.0.0' });

      await visit('/crates/foo');
      assert.equal(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('2.0.0');
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('defaults to the highest stable version, even if there are higher prereleases', async function (assert) {
      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '1.0.0' });
      this.server.create('version', { crate, num: '1.2.3', yanked: true });
      this.server.create('version', { crate, num: '2.0.0-beta.1' });

      await visit('/crates/foo');
      assert.equal(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('1.0.0');
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('defaults to the highest not-yanked version', async function (assert) {
      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '1.0.0', yanked: true });
      this.server.create('version', { crate, num: '1.2.3', yanked: true });
      this.server.create('version', { crate, num: '2.0.0-beta.1' });
      this.server.create('version', { crate, num: '2.0.0-beta.2' });
      this.server.create('version', { crate, num: '2.0.0', yanked: true });

      await visit('/crates/foo');
      assert.equal(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('2.0.0-beta.2');
      assert.dom('[data-test-notification-message]').doesNotExist();
    });

    test('if there are only yanked versions, it defaults to the latest version', async function (assert) {
      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '1.0.0', yanked: true });
      this.server.create('version', { crate, num: '1.2.3', yanked: true });
      this.server.create('version', { crate, num: '2.0.0-beta.1', yanked: true });

      await visit('/crates/foo');
      assert.equal(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('2.0.0-beta.1');
      assert.dom('[data-test-notification-message]').doesNotExist();
    });
  });
});
