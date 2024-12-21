import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../helpers/visit-ignoring-abort';

module('Route | crate.range', function (hooks) {
  setupApplicationTest(hooks);

  test('happy path', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.2.0' });
    this.server.create('version', { crate, num: '1.2.3' });

    await visit('/crates/foo/range/^1.1.0');
    assert.strictEqual(currentURL(), `/crates/foo/1.2.3`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('v1.2.3');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('happy path with tilde range', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.1.1' });
    this.server.create('version', { crate, num: '1.2.0' });

    await visit('/crates/foo/range/~1.1.0');
    assert.strictEqual(currentURL(), `/crates/foo/1.1.1`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('v1.1.1');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('happy path with cargo style and', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.4.2' });
    this.server.create('version', { crate, num: '1.3.4' });
    this.server.create('version', { crate, num: '1.3.3' });
    this.server.create('version', { crate, num: '1.2.6' });

    await visit('/crates/foo/range/>=1.3.0, <1.4.0');
    assert.strictEqual(currentURL(), `/crates/foo/1.3.4`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('v1.3.4');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('ignores yanked versions if possible', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.1.1' });
    this.server.create('version', { crate, num: '1.2.0', yanked: true });

    await visit('/crates/foo/range/^1.0.0');
    assert.strictEqual(currentURL(), `/crates/foo/1.1.1`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('v1.1.1');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('falls back to yanked version if necessary', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0', yanked: true });
    this.server.create('version', { crate, num: '1.1.0', yanked: true });
    this.server.create('version', { crate, num: '1.1.1', yanked: true });
    this.server.create('version', { crate, num: '2.0.0' });

    await visit('/crates/foo/range/^1.0.0');
    assert.strictEqual(currentURL(), `/crates/foo/1.1.1`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('v1.1.1');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('shows an error page if crate not found', async function (assert) {
    await visit('/crates/foo/range/^3');
    assert.strictEqual(currentURL(), '/crates/foo/range/%5E3');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Crate not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('shows an error page if crate fails to load', async function (assert) {
    this.server.get('/api/v1/crates/:crate_name', {}, 500);

    await visit('/crates/foo/range/^3');
    assert.strictEqual(currentURL(), '/crates/foo/range/%5E3');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load crate data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('shows an error page if no match found', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.1.1' });
    this.server.create('version', { crate, num: '2.0.0' });

    await visit('/crates/foo/range/^3');
    assert.strictEqual(currentURL(), '/crates/foo/range/%5E3');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: No matching version found for ^3');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('shows an error page if versions fail to load', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '3.2.1' });

    this.server.get('/api/v1/crates/:crate_name/versions', {}, 500);

    // Load `crate` and ensure all `versions` are not loaded.
    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', 'foo');
    let versionsRef = crateRecord.hasMany('versions');
    assert.deepEqual(versionsRef.ids(), []);

    await visit('/crates/foo/range/^3');
    assert.strictEqual(currentURL(), '/crates/foo/range/%5E3');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load version data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });
});
