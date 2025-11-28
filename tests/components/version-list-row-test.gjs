import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import Row from 'crates-io/components/version-list/row';
import { setupRenderingTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Component | VersionList::Row', function (hooks) {
  setupRenderingTest(hooks);
  setupMsw(hooks);

  test('handle non-standard semver strings', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo' });
    await this.db.version.create({ crate, num: '0.4.0-alpha.01' });
    await this.db.version.create({ crate, num: '0.3.0-alpha.01' });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.loadVersionsTask.perform()).slice();
    await crateRecord.loadOwnerUserTask.perform();
    let firstVersion = versions.find(it => it.num === '0.4.0-alpha.01');
    let secondVersion = versions.find(it => it.num === '0.3.0-alpha.01');

    await render(<template><Row @version={{firstVersion}} /></template>);
    assert.dom('[data-test-release-track]').hasText('0.4');
    assert.dom('[data-test-release-track-link]').hasText('0.4.0-alpha.01');

    await render(<template><Row @version={{secondVersion}} /></template>);
    assert.dom('[data-test-release-track]').hasText('0.3');
    assert.dom('[data-test-release-track-link]').hasText('0.3.0-alpha.01');
  });

  test('handle node-semver parsing errors', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo' });
    let version = '18446744073709551615.18446744073709551615.18446744073709551615';
    await this.db.version.create({ crate, num: version });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    let versionModel = (await crateRecord.loadVersionsTask.perform()).slice()[0];
    await crateRecord.loadOwnerUserTask.perform();

    await render(<template><Row @version={{versionModel}} /></template>);
    assert.dom('[data-test-release-track]').hasText('?');
    assert.dom('[data-test-release-track-link]').hasText(version);
  });

  test('pluralize "feature" only when appropriate', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo' });
    await this.db.version.create({
      crate,
      num: '0.1.0',
      features: {},
    });
    await this.db.version.create({
      crate,
      num: '0.2.0',
      features: { one: [] },
    });
    await this.db.version.create({
      crate,
      num: '0.3.0',
      features: { one: [], two: [] },
    });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.loadVersionsTask.perform()).slice();
    await crateRecord.loadOwnerUserTask.perform();
    let firstVersion = versions.find(it => it.num === '0.1.0');
    let secondVersion = versions.find(it => it.num === '0.2.0');
    let thirdVersion = versions.find(it => it.num === '0.3.0');

    await render(<template><Row @version={{firstVersion}} /></template>);
    assert.dom('[data-test-feature-list]').doesNotExist();

    await render(<template><Row @version={{secondVersion}} /></template>);
    assert.dom('[data-test-feature-list]').hasText('1 Feature');

    await render(<template><Row @version={{thirdVersion}} /></template>);
    assert.dom('[data-test-feature-list]').hasText('2 Features');
  });
});
