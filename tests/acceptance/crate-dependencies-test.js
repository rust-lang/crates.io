import { currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { getPageTitle } from 'ember-page-title/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | crate dependencies page', function (hooks) {
  setupApplicationTest(hooks);

  test('shows the lists of dependencies', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/dependencies');
    assert.equal(currentURL(), '/crates/nanomsg/0.6.1/dependencies');
    assert.equal(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

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

  test('shows error message if loading of dependencies fails', async function (assert) {
    this.server.loadFixtures();

    this.server.get('/api/v1/crates/:crate_name/:version_num/dependencies', {}, 500);

    await visit('/crates/nanomsg/dependencies');
    assert.equal(currentURL(), '/crates/nanomsg');

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText("Failed to load the list of dependencies for the 'nanomsg' crate. Please try again later!");
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
    assert.equal(currentURL(), '/crates/nanomsg/0.6.1/dependencies');

    assert.dom('[data-test-dependencies] li').exists({ count: 2 });

    assert.dom('[data-test-dependency="foo"]').exists();
    assert.dom('[data-test-dependency="foo"] [data-test-crate-name]').hasText('foo');
    assert.dom('[data-test-dependency="bar"] [data-test-description]').doesNotExist();

    assert.dom('[data-test-dependency="bar"]').exists();
    assert.dom('[data-test-dependency="bar"] [data-test-crate-name]').hasText('bar');
    assert.dom('[data-test-dependency="bar"] [data-test-description]').doesNotExist();
  });
});
