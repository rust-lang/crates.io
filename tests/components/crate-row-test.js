import { render } from '@ember/test-helpers';
import { setupRenderingTest } from 'ember-qunit';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import setupMirage from '../helpers/setup-mirage';

module('Component | CrateRow', function (hooks) {
  setupRenderingTest(hooks);
  setupMirage(hooks);

  test('shows crate name and highest version', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('version', { crate, num: '1.2.3', yanked: true });
    this.server.create('version', { crate, num: '2.0.0-beta.1' });
    this.server.create('version', { crate, num: '1.1.2' });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);

    await render(hbs`<CrateRow @crate={{this.crate}} />`);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').hasText('v2.0.0-beta.1');
    assert.dom('[data-test-copy-toml-button]').exists();
  });

  test('shows crate name and `0.0.0` version if all versions are yanked', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0', yanked: true });
    this.server.create('version', { crate, num: '1.2.3', yanked: true });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);

    await render(hbs`<CrateRow @crate={{this.crate}} />`);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').hasText('v0.0.0');
    assert.dom('[data-test-copy-toml-button]').exists();
  });
});
