import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Component | VersionList::Row', function (hooks) {
  setupRenderingTest(hooks);
  setupMsw(hooks);

  test('handle non-standard semver strings', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '0.4.0-alpha.01', created_at: Date.now(), updated_at: Date.now() });
    this.db.version.create({ crate, num: '0.3.0-alpha.01', created_at: Date.now(), updated_at: Date.now() });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.loadVersionsTask.perform()).slice();
    await crateRecord.loadOwnerUserTask.perform();
    this.firstVersion = versions.find(it => it.num === '0.4.0-alpha.01');
    this.secondVersion = versions.find(it => it.num === '0.3.0-alpha.01');

    await render(hbs`<VersionList::Row @version={{this.firstVersion}} />`);
    assert.dom('[data-test-release-track]').hasText('0.4');
    assert.dom('[data-test-release-track-link]').hasText('0.4.0-alpha.01');

    await render(hbs`<VersionList::Row @version={{this.secondVersion}} />`);
    assert.dom('[data-test-release-track]').hasText('0.3');
    assert.dom('[data-test-release-track-link]').hasText('0.3.0-alpha.01');
  });

  test('handle node-semver parsing errors', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    let version = '18446744073709551615.18446744073709551615.18446744073709551615';
    this.db.version.create({ crate, num: version });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    this.version = (await crateRecord.loadVersionsTask.perform()).slice()[0];
    await crateRecord.loadOwnerUserTask.perform();

    await render(hbs`<VersionList::Row @version={{this.version}} />`);
    assert.dom('[data-test-release-track]').hasText('?');
    assert.dom('[data-test-release-track-link]').hasText(version);
  });

  test('pluralize "feature" only when appropriate', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({
      crate,
      num: '0.1.0',
      features: {},
      created_at: Date.now(),
      updated_at: Date.now(),
    });
    this.db.version.create({
      crate,
      num: '0.2.0',
      features: { one: [] },
      created_at: Date.now(),
      updated_at: Date.now(),
    });
    this.db.version.create({
      crate,
      num: '0.3.0',
      features: { one: [], two: [] },
      created_at: Date.now(),
      updated_at: Date.now(),
    });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.loadVersionsTask.perform()).slice();
    await crateRecord.loadOwnerUserTask.perform();
    this.firstVersion = versions.find(it => it.num === '0.1.0');
    this.secondVersion = versions.find(it => it.num === '0.2.0');
    this.thirdVersion = versions.find(it => it.num === '0.3.0');

    await render(hbs`<VersionList::Row @version={{this.firstVersion}} />`);
    assert.dom('[data-test-feature-list]').doesNotExist();

    await render(hbs`<VersionList::Row @version={{this.secondVersion}} />`);
    assert.dom('[data-test-feature-list]').hasText('1 Feature');

    await render(hbs`<VersionList::Row @version={{this.thirdVersion}} />`);
    assert.dom('[data-test-feature-list]').hasText('2 Features');
  });
});
