import { currentURL, visit } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from '../../../helpers/setup-mirage';

module('Route | crate.version | model() hook', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

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

    test('redirects to unspecific version URL', async function (assert) {
      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '1.0.0' });
      this.server.create('version', { crate, num: '1.2.3', yanked: true });
      this.server.create('version', { crate, num: '2.0.0-beta.1' });

      await visit('/crates/foo/2.0.0');
      assert.equal(currentURL(), `/crates/foo`);
      assert.dom('[data-test-crate-name]').hasText('foo');
      assert.dom('[data-test-crate-version]').hasText('1.0.0');
      assert.dom('[data-test-notification-message="error"]').hasText("Version '2.0.0' of crate 'foo' does not exist");
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
