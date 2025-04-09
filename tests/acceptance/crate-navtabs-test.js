import { click, currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

const TAB_README = '[data-test-readme-tab] a';
const TAB_VERSIONS = '[data-test-versions-tab] a';
const TAB_DEPS = '[data-test-deps-tab] a';
const TAB_REV_DEPS = '[data-test-rev-deps-tab] a';
const TAB_SETTINGS = '[data-test-settings-tab] a';

module('Acceptance | crate navigation tabs', function (hooks) {
  setupApplicationTest(hooks);

  test('basic navigation between tabs works as expected', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate, num: '0.6.1' });

    await visit('/crates/nanomsg');
    assert.strictEqual(currentURL(), '/crates/nanomsg');

    assert.dom(TAB_README).hasAttribute('href', '/crates/nanomsg').hasAttribute('data-test-active');
    assert.dom(TAB_VERSIONS).hasAttribute('href', '/crates/nanomsg/versions').hasNoAttribute('data-test-active');
    assert.dom(TAB_DEPS).hasAttribute('href', '/crates/nanomsg/dependencies').hasNoAttribute('data-test-active');
    assert
      .dom(TAB_REV_DEPS)
      .hasAttribute('href', '/crates/nanomsg/reverse_dependencies')
      .hasNoAttribute('data-test-active');
    assert.dom(TAB_SETTINGS).doesNotExist();

    await click(TAB_VERSIONS);
    assert.strictEqual(currentURL(), '/crates/nanomsg/versions');

    assert.dom(TAB_README).hasAttribute('href', '/crates/nanomsg').hasNoAttribute('data-test-active');
    assert.dom(TAB_VERSIONS).hasAttribute('href', '/crates/nanomsg/versions').hasAttribute('data-test-active');
    assert.dom(TAB_DEPS).hasAttribute('href', '/crates/nanomsg/dependencies').hasNoAttribute('data-test-active');
    assert
      .dom(TAB_REV_DEPS)
      .hasAttribute('href', '/crates/nanomsg/reverse_dependencies')
      .hasNoAttribute('data-test-active');
    assert.dom(TAB_SETTINGS).doesNotExist();

    await click(TAB_DEPS);
    assert.strictEqual(currentURL(), '/crates/nanomsg/0.6.1/dependencies');

    assert.dom(TAB_README).hasAttribute('href', '/crates/nanomsg/0.6.1').hasNoAttribute('data-test-active');
    assert.dom(TAB_VERSIONS).hasAttribute('href', '/crates/nanomsg/versions').hasNoAttribute('data-test-active');
    assert.dom(TAB_DEPS).hasAttribute('href', '/crates/nanomsg/0.6.1/dependencies').hasAttribute('data-test-active');
    assert
      .dom(TAB_REV_DEPS)
      .hasAttribute('href', '/crates/nanomsg/reverse_dependencies')
      .hasNoAttribute('data-test-active');
    assert.dom(TAB_SETTINGS).doesNotExist();

    await click(TAB_REV_DEPS);
    assert.strictEqual(currentURL(), '/crates/nanomsg/reverse_dependencies');

    assert.dom(TAB_README).hasAttribute('href', '/crates/nanomsg').hasNoAttribute('data-test-active');
    assert.dom(TAB_VERSIONS).hasAttribute('href', '/crates/nanomsg/versions').hasNoAttribute('data-test-active');
    assert.dom(TAB_DEPS).hasAttribute('href', '/crates/nanomsg/dependencies').hasNoAttribute('data-test-active');
    assert
      .dom(TAB_REV_DEPS)
      .hasAttribute('href', '/crates/nanomsg/reverse_dependencies')
      .hasAttribute('data-test-active');
    assert.dom(TAB_SETTINGS).doesNotExist();
  });
});
