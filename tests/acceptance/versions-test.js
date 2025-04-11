import { click, currentURL, findAll, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Acceptance | crate versions page', function (hooks) {
  setupApplicationTest(hooks);

  test('show versions sorted by date', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate, num: '0.1.0', created_at: '2017-01-01' });
    this.db.version.create({ crate, num: '0.2.0', created_at: '2018-01-01' });
    this.db.version.create({ crate, num: '0.3.0', created_at: '2019-01-01', rust_version: '1.69' });
    this.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await visit('/crates/nanomsg/versions');
    assert.strictEqual(currentURL(), '/crates/nanomsg/versions');

    let versions = findAll('[data-test-version]').map(it => it.dataset.testVersion);
    assert.deepEqual(versions, ['0.2.1', '0.3.0', '0.2.0', '0.1.0']);

    await percySnapshot(assert);

    await click('[data-test-current-order]');
    await click('[data-test-semver-sort] a');

    versions = findAll('[data-test-version]').map(it => it.dataset.testVersion);
    assert.deepEqual(versions, ['0.3.0', '0.2.1', '0.2.0', '0.1.0']);
  });

  test('shows correct release tracks label after yanking/unyanking', async function (assert) {
    let user = this.db.user.create();
    this.authenticateAs(user);

    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.crateOwnership.create({ crate, user });

    this.db.version.create({ crate, num: '0.1.0', created_at: '2017-01-01' });
    this.db.version.create({ crate, num: '0.2.0', created_at: '2018-01-01' });
    this.db.version.create({ crate, num: '0.3.0', created_at: '2019-01-01', rust_version: '1.69' });
    this.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await visit('/crates/nanomsg/versions');
    assert.strictEqual(currentURL(), '/crates/nanomsg/versions');

    let versions = findAll('[data-test-version]').map(it => it.dataset.testVersion);
    assert.deepEqual(versions, ['0.2.1', '0.3.0', '0.2.0', '0.1.0']);

    assert
      .dom('[data-test-version="0.2.1"]')
      .hasClass(/.*latest/)
      .hasNoClass(/.yanked/);
    assert
      .dom('[data-test-version="0.2.0"]')
      .hasNoClass(/.*latest/)
      .hasNoClass(/.yanked/);

    // yanking
    await click('[data-test-version-yank-button="0.2.1"]');
    assert
      .dom('[data-test-version="0.2.1"]')
      .hasNoClass(/.*latest/)
      .hasClass(/.*yanked/);
    assert
      .dom('[data-test-version="0.2.0"]')
      .hasClass(/.*latest/)
      .hasNoClass(/.*yanked/);

    // unyanking
    await click('[data-test-version-unyank-button="0.2.1"]');
    assert
      .dom('[data-test-version="0.2.1"]')
      .hasClass(/.*latest/)
      .hasNoClass(/.yanked/);
    assert
      .dom('[data-test-version="0.2.0"]')
      .hasNoClass(/.*latest/)
      .hasNoClass(/.yanked/);
  });
});
