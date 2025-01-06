import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { getPageTitle } from 'ember-page-title/test-support';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../axe-config';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | crate dependencies page', function (hooks) {
  setupApplicationTest(hooks);

  test('shows the lists of dependencies', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/dependencies');
    assert.strictEqual(currentURL(), '/crates/nanomsg/0.6.1/dependencies');
    assert.strictEqual(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-dependencies] li').exists({ count: 2 });
    assert.dom('[data-test-build-dependencies] li').exists({ count: 1 });
    assert.dom('[data-test-dev-dependencies] li').exists({ count: 1 });

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('empty list case', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '0.6.1' });

    await visit('/crates/nanomsg/dependencies');

    assert.dom('[data-test-no-dependencies]').exists();
    assert.dom('[data-test-dependencies] li').doesNotExist();
    assert.dom('[data-test-build-dependencies] li').doesNotExist();
    assert.dom('[data-test-dev-dependencies] li').doesNotExist();
  });

  test('shows an error page if crate not found', async function (assert) {
    await visit('/crates/foo/1.0.0/dependencies');
    assert.strictEqual(currentURL(), '/crates/foo/1.0.0/dependencies');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Crate not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('shows an error page if crate fails to load', async function (assert) {
    this.server.get('/api/v1/crates/:crate_name', {}, 500);

    await visit('/crates/foo/1.0.0/dependencies');
    assert.strictEqual(currentURL(), '/crates/foo/1.0.0/dependencies');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load crate data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('shows an error page if version is not found', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '2.0.0' });

    await visit('/crates/foo/1.0.0/dependencies');
    assert.strictEqual(currentURL(), '/crates/foo/1.0.0/dependencies');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Version 1.0.0 not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('shows an error page if versions fail to load', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '2.0.0' });

    this.server.get('/api/v1/crates/:crate_name/versions', {}, 500);

    await visit('/crates/foo/1.0.0/dependencies');
    assert.strictEqual(currentURL(), '/crates/foo/1.0.0/dependencies');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load version data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('shows error message if loading of dependencies fails', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('/api/v1/crates/:crate_name/:version_num/dependencies', {}, 500);

    await visit('/crates/foo/1.0.0/dependencies');
    assert.strictEqual(currentURL(), '/crates/foo/1.0.0/dependencies');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load dependencies');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('hides description if loading of dependency details fails', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg' });
    let version = this.server.create('version', { crate, num: '0.6.1' });

    let foo = this.server.create('crate', { name: 'foo', description: 'This is the foo crate' });
    this.server.create('version', { crate: foo, num: '1.0.0' });
    this.server.create('dependency', { crate: foo, version, req: '^1.0.0', kind: 'normal' });

    let bar = this.server.create('crate', { name: 'bar', description: 'This is the bar crate' });
    this.server.create('version', { crate: bar, num: '2.3.4' });
    this.server.create('dependency', { crate: bar, version, req: '^2.0.0', kind: 'normal' });

    this.server.get('/api/v1/crates', {}, 500);

    await visit('/crates/nanomsg/dependencies');
    assert.strictEqual(currentURL(), '/crates/nanomsg/0.6.1/dependencies');

    assert.dom('[data-test-dependencies] li').exists({ count: 2 });

    assert.dom('[data-test-dependency="foo"]').exists();
    assert.dom('[data-test-dependency="foo"] [data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-dependency="bar"] [data-test-description]').doesNotExist();

    assert.dom('[data-test-dependency="bar"]').exists();
    assert.dom('[data-test-dependency="bar"] [data-test-crate-name]').hasText('bar');
    assert.dom('[data-test-dependency="bar"] [data-test-description]').doesNotExist();
  });
});
