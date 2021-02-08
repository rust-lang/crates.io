import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import setupMirage from 'ember-cli-mirage/test-support/setup-mirage';

module('Model | Version', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  module('semver', function () {
    async function prepare(context, { num }) {
      let { server, store } = context;

      let crate = server.create('crate');
      server.create('version', { crate, num });

      let crateRecord = await store.findRecord('crate', crate.id);
      let versions = (await crateRecord.versions).toArray();
      return versions[0];
    }

    test('parses 1.2.3 correctly', async function (assert) {
      let { semver, releaseTrack, isPrerelease } = await prepare(this, { num: '1.2.3' });
      assert.strictEqual(semver.major, 1);
      assert.strictEqual(semver.minor, 2);
      assert.strictEqual(semver.patch, 3);
      assert.deepEqual(semver.prerelease, []);
      assert.deepEqual(semver.build, []);
      assert.false(isPrerelease);
      assert.strictEqual(releaseTrack, '1.x');
    });

    test('parses 0.3.1-rc.1 correctly', async function (assert) {
      let { semver, releaseTrack, isPrerelease } = await prepare(this, { num: '0.3.1-rc.1' });
      assert.strictEqual(semver.major, 0);
      assert.strictEqual(semver.minor, 3);
      assert.strictEqual(semver.patch, 1);
      assert.deepEqual(semver.prerelease, ['rc', 1]);
      assert.deepEqual(semver.build, []);
      assert.true(isPrerelease);
      assert.strictEqual(releaseTrack, '0.3');
    });

    test('parses 0.0.0-alpha.1+1234 correctly', async function (assert) {
      let { semver, releaseTrack, isPrerelease } = await prepare(this, { num: '0.0.0-alpha.1+1234' });
      assert.strictEqual(semver.major, 0);
      assert.strictEqual(semver.minor, 0);
      assert.strictEqual(semver.patch, 0);
      assert.deepEqual(semver.prerelease, ['alpha', 1]);
      assert.deepEqual(semver.build, ['1234']);
      assert.true(isPrerelease);
      assert.strictEqual(releaseTrack, '0.0');
    });
  });

  module('featuresList', function () {
    async function prepare(context, { features }) {
      let { server, store } = context;

      let crate = server.create('crate');
      server.create('version', { crate, features });

      let crateRecord = await store.findRecord('crate', crate.id);
      let versions = (await crateRecord.versions).toArray();
      return versions[0];
    }

    test('`features: {}` results in empty list', async function (assert) {
      let version = await prepare(this, { features: {} });
      assert.deepEqual(version.featureList, []);
    });

    test('`features: null` results in empty list', async function (assert) {
      let version = await prepare(this, { features: null });
      assert.deepEqual(version.featureList, []);
    });

    test('real world case', async function (assert) {
      let features = {
        alloc: ['rand_core/alloc'],
        default: ['std', 'std_rng'],
        getrandom: ['rand_core/getrandom'],
        nightly: [],
        serde1: ['serde'],
        simd_support: ['packed_simd'],
        small_rng: [],
        std: ['rand_core/std', 'rand_chacha/std', 'alloc', 'getrandom', 'libc'],
        std_rng: ['rand_chacha', 'rand_hc'],
      };

      let version = await prepare(this, { features });
      assert.deepEqual(version.featureList, [
        {
          dependencies: ['rand_core/alloc'],
          isDefault: false,
          name: 'alloc',
        },
        {
          dependencies: ['rand_core/getrandom'],
          isDefault: false,
          name: 'getrandom',
        },
        {
          dependencies: [],
          isDefault: false,
          name: 'nightly',
        },
        {
          dependencies: ['serde'],
          isDefault: false,
          name: 'serde1',
        },
        {
          dependencies: ['packed_simd'],
          isDefault: false,
          name: 'simd_support',
        },
        {
          dependencies: [],
          isDefault: false,
          name: 'small_rng',
        },
        {
          dependencies: ['rand_core/std', 'rand_chacha/std', 'alloc', 'getrandom', 'libc'],
          isDefault: true,
          name: 'std',
        },
        {
          dependencies: ['rand_chacha', 'rand_hc'],
          isDefault: true,
          name: 'std_rng',
        },
      ]);
    });
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
