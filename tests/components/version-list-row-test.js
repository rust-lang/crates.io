import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

import setupMirage from '../helpers/setup-mirage';

module('Component | VersionList::Row', function (hooks) {
  setupRenderingTest(hooks);
  setupMirage(hooks);

  test('handle non-standard semver strings', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '0.4.0-alpha.01', created_at: Date.now(), updated_at: Date.now() });
    this.server.create('version', { crate, num: '0.3.0-alpha.01', created_at: Date.now(), updated_at: Date.now() });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.versions).toArray();
    this.firstVersion = versions[0];
    this.secondVersion = versions[1];

    await render(hbs`<VersionList::Row @version={{this.firstVersion}} />`);
    assert.dom('[data-test-release-track] svg').exists();
    assert.dom('[data-test-release-track-link]').hasText('0.4.0-alpha.01');

    await render(hbs`<VersionList::Row @version={{this.secondVersion}} />`);
    assert.dom('[data-test-release-track]').hasText('0.3');
    assert.dom('[data-test-release-track-link]').hasText('0.3.0-alpha.01');
  });

  test('handle node-semver parsing errors', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    let version = '18446744073709551615.18446744073709551615.18446744073709551615';
    this.server.create('version', { crate, num: version });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    this.version = (await crateRecord.versions).toArray()[0];

    await render(hbs`<VersionList::Row @version={{this.version}} />`);
    assert.dom('[data-test-release-track]').hasText('?');
    assert.dom('[data-test-release-track-link]').hasText(version);
  });

  test('pluralize "feature" only when appropriate', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', {
      crate,
      num: '0.1.0',
      features: {},
      created_at: Date.now(),
      updated_at: Date.now(),
    });
    this.server.create('version', {
      crate,
      num: '0.2.0',
      features: { one: [] },
      created_at: Date.now(),
      updated_at: Date.now(),
    });
    this.server.create('version', {
      crate,
      num: '0.3.0',
      features: { one: [], two: [] },
      created_at: Date.now(),
      updated_at: Date.now(),
    });

    let store = this.owner.lookup('service:store');
    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.versions).toArray();
    this.firstVersion = versions[0];
    this.secondVersion = versions[1];
    this.thirdVersion = versions[2];

    await render(hbs`<VersionList::Row @version={{this.firstVersion}} />`);
    assert.dom('[data-test-feature-list]').doesNotExist();

    await render(hbs`<VersionList::Row @version={{this.secondVersion}} />`);
    assert.dom('[data-test-feature-list]').hasText('1 Feature');

    await render(hbs`<VersionList::Row @version={{this.thirdVersion}} />`);
    assert.dom('[data-test-feature-list]').hasText('2 Features');
  });
});
