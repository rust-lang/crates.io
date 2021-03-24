import { currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Route | crate.range', function (hooks) {
  setupApplicationTest(hooks);

  test('happy path', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.2.0' });
    this.server.create('version', { crate, num: '1.2.3' });

    await visit('/crates/foo/range/^1.1.0');
    assert.equal(currentURL(), `/crates/foo/1.2.3`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('1.2.3');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('happy path with tilde range', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.1.1' });
    this.server.create('version', { crate, num: '1.2.0' });

    await visit('/crates/foo/range/~1.1.0');
    assert.equal(currentURL(), `/crates/foo/1.1.1`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('1.1.1');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('happy path with cargo style and', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.4.2' });
    this.server.create('version', { crate, num: '1.3.4' });
    this.server.create('version', { crate, num: '1.3.3' });
    this.server.create('version', { crate, num: '1.2.6' });

    await visit('/crates/foo/range/>=1.3.0, <1.4.0');
    assert.equal(currentURL(), `/crates/foo/1.3.4`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('1.3.4');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('ignores yanked versions if possible', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.1.1' });
    this.server.create('version', { crate, num: '1.2.0', yanked: true });

    await visit('/crates/foo/range/^1.0.0');
    assert.equal(currentURL(), `/crates/foo/1.1.1`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('1.1.1');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('falls back to yanked version if necessary', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0', yanked: true });
    this.server.create('version', { crate, num: '1.1.0', yanked: true });
    this.server.create('version', { crate, num: '1.1.1', yanked: true });
    this.server.create('version', { crate, num: '2.0.0' });

    await visit('/crates/foo/range/^1.0.0');
    assert.equal(currentURL(), `/crates/foo/1.1.1`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('1.1.1');
    assert.dom('[data-test-notification-message]').doesNotExist();
  });

  test('redirects to main crate page if no match found', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.1.0' });
    this.server.create('version', { crate, num: '1.1.1' });
    this.server.create('version', { crate, num: '2.0.0' });

    await visit('/crates/foo/range/^3');
    assert.equal(currentURL(), `/crates/foo`);
    assert.dom('[data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-crate-version]').hasText('2.0.0');
    assert.dom('[data-test-notification-message="error"]').hasText("No matching version of crate 'foo' found for: ^3");
  });
});
