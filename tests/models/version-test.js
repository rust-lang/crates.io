import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from 'ember-cli-mirage/test-support/setup-mirage';

module('Model | Version', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  test('`published_by` relationship is assigned correctly', async function (assert) {
    let user = this.server.create('user', { name: 'JD' });

    let crate = this.server.create('crate');
    this.server.create('version', { crate, publishedBy: user });

    let crateRecord = await this.store.findRecord('crate', crate.id);
    assert.ok(crateRecord);
    let versions = (await crateRecord.versions).toArray();
    assert.equal(versions.length, 1);
    let version = versions[0];
    assert.ok(version.published_by);
    assert.equal(version.published_by.name, 'JD');
  });
});
