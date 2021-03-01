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
    this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.1' });

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
});
