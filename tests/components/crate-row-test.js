import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

import setupMirage from '../helpers/setup-mirage';

module('Component | CrateRow', function (hooks) {
  setupRenderingTest(hooks);
  setupMirage(hooks);

  test('shows crate name and highest stable version', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.2.3', yanked: true });
    this.server.create('version', { crate, num: '2.0.0-beta.1' });
    this.server.create('version', { crate, num: '1.1.2' });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);

    await render(hbs`<CrateRow @crate={{this.crate}} />`);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').hasText('v1.1.2');
    assert.dom('[data-test-copy-toml-button]').exists();
  });

  test('shows crate name and highest version, if there is no stable version available', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0-beta.1' });
    this.server.create('version', { crate, num: '1.0.0-beta.3' });
    this.server.create('version', { crate, num: '1.0.0-beta.2' });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);

    await render(hbs`<CrateRow @crate={{this.crate}} />`);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').hasText('v1.0.0-beta.3');
    assert.dom('[data-test-copy-toml-button]').exists();
  });

  test('shows crate name and no version if all versions are yanked', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0', yanked: true });
    this.server.create('version', { crate, num: '1.2.3', yanked: true });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);

    await render(hbs`<CrateRow @crate={{this.crate}} />`);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').doesNotExist();
    assert.dom('[data-test-copy-toml-button]').doesNotExist();
  });
});
