import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import CrateSidebar from 'crates-io/components/crate-sidebar';
import { setupRenderingTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Component | CrateSidebar | toml snippet', function (hooks) {
  setupRenderingTest(hooks);
  setupMsw(hooks);

  test('show version number with `=` prefix', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0' });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    let version = (await crateModel.versions).slice()[0];
    await crateModel.loadOwnersTask.perform();

    await render(<template><CrateSidebar @crate={{crateModel}} @version={{version}} /></template>);
    assert.dom('[title="Copy command to clipboard"]').exists().hasText('cargo add foo');
    assert.dom('[title="Copy Cargo.toml snippet to clipboard"]').exists().hasText('foo = "1.0.0"');

    await render(
      <template><CrateSidebar @crate={{crateModel}} @version={{version}} @requestedVersion='1.0.0' /></template>,
    );
    assert.dom('[title="Copy command to clipboard"]').exists().hasText('cargo add foo@=1.0.0');
    assert.dom('[title="Copy Cargo.toml snippet to clipboard"]').exists().hasText('foo = "=1.0.0"');
  });

  test('show version number without build metadata', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0+abcdef' });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    let version = (await crateModel.versions).slice()[0];
    await crateModel.loadOwnersTask.perform();

    await render(<template><CrateSidebar @crate={{crateModel}} @version={{version}} /></template>);
    assert.dom('[title="Copy Cargo.toml snippet to clipboard"]').exists().hasText('foo = "1.0.0"');
  });

  test('show pre-release version number without build', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0-alpha+abcdef' });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    let version = (await crateModel.versions).slice()[0];
    await crateModel.loadOwnersTask.perform();

    await render(<template><CrateSidebar @crate={{crateModel}} @version={{version}} /></template>);
    assert.dom('[title="Copy Cargo.toml snippet to clipboard"]').exists().hasText('foo = "1.0.0-alpha"');
  });
});
