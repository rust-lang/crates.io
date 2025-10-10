import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import CrateRow from 'crates-io/components/crate-row';
import { setupRenderingTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Component | CrateRow', function (hooks) {
  setupRenderingTest(hooks);
  setupMsw(hooks);

  test('shows crate name and highest stable version', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0' });
    this.db.version.create({ crate, num: '1.2.3', yanked: true });
    this.db.version.create({ crate, num: '2.0.0-beta.1' });
    this.db.version.create({ crate, num: '1.1.2' });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);

    await render(<template><CrateRow @crate={{crateModel}} /></template>);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').hasText('v1.1.2');
    assert.dom('[data-test-copy-toml-button]').exists();
  });

  test('shows crate name and highest version, if there is no stable version available', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0-beta.1' });
    this.db.version.create({ crate, num: '1.0.0-beta.3' });
    this.db.version.create({ crate, num: '1.0.0-beta.2' });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);

    await render(<template><CrateRow @crate={{crateModel}} /></template>);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').hasText('v1.0.0-beta.3');
    assert.dom('[data-test-copy-toml-button]').exists();
  });

  test('shows crate name and no version if all versions are yanked', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0', yanked: true });
    this.db.version.create({ crate, num: '1.2.3', yanked: true });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);

    await render(<template><CrateRow @crate={{crateModel}} /></template>);
    assert.dom('[data-test-crate-link]').hasText('foo');
    assert.dom('[data-test-version]').doesNotExist();
    assert.dom('[data-test-copy-toml-button]').doesNotExist();
  });
});
