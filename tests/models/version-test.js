import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import { setupMirage } from 'ember-cli-mirage/test-support';

import { setupFakeTimers } from '../helpers/fake-timers';

module('Model | Version', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);
  setupFakeTimers(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  test('isNew', async function (assert) {
    let { server, store } = this;

    let crate = server.create('crate');
    server.create('version', { crate, created_at: '2010-06-16T21:30:45Z' });

    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.versions).toArray();

    this.clock.setSystemTime(new Date('2010-06-16T21:40:45Z'));
    assert.true(versions[0].isNew);

    this.clock.setSystemTime(new Date('2010-06-23T21:40:45Z'));
    assert.true(versions[0].isNew);

    this.clock.setSystemTime(new Date('2010-06-24T21:40:45Z'));
    assert.false(versions[0].isNew);

    this.clock.setSystemTime(new Date('2014-06-24T21:40:45Z'));
    assert.false(versions[0].isNew);
  });

  module('semver', function () {
    async function prepare(context, { num }) {
      let { server, store } = context;

      let crate = server.create('crate');
      server.create('version', { crate, num });

      let crateRecord = await store.findRecord('crate', crate.name);
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

    test('123.0.0 has 123 release track', async function (assert) {
      let { semver, releaseTrack, isPrerelease } = await prepare(this, { num: '123.0.0' });
      assert.strictEqual(semver.major, 123);
      assert.strictEqual(semver.minor, 0);
      assert.strictEqual(semver.patch, 0);
      assert.deepEqual(semver.prerelease, []);
      assert.deepEqual(semver.build, []);
      assert.false(isPrerelease);
      assert.strictEqual(releaseTrack, '123');
    });

    test('parses 0.3.0-alpha.01 (non-standard) correctly', async function (assert) {
      let { semver, releaseTrack, isPrerelease } = await prepare(this, { num: '0.3.0-alpha.01' });
      assert.strictEqual(semver.major, 0);
      assert.strictEqual(semver.minor, 3);
      assert.strictEqual(semver.patch, 0);
      assert.deepEqual(semver.prerelease, ['alpha', 1]);
      assert.true(isPrerelease);
      assert.strictEqual(releaseTrack, '0.3');
    });

    test('invalidSemver is true for unparseable versions', async function (assert) {
      let { invalidSemver } = await prepare(this, {
        num: '18446744073709551615.18446744073709551615.1844674407370955161',
      });
      assert.true(invalidSemver);
    });
  });

  module('isHighestOfReleaseTrack', function () {
    test('happy path', async function (assert) {
      let nums = [
        '0.4.0-rc.1',
        '0.3.24-alpha.02',
        '0.3.23',
        '0.3.22',
        '0.3.21-pre.0',
        '0.3.20',
        '0.3.3',
        '0.3.2',
        '0.3.1',
        '0.3.0-alpha.01',
        '0.3.0',
        '0.2.1',
        '0.2.0',
        '0.1.2',
        '0.1.1',
      ];

      let crate = this.server.create('crate');
      for (let num of nums) {
        this.server.create('version', { crate, num });
      }

      let crateRecord = await this.store.findRecord('crate', crate.name);
      let versions = (await crateRecord.versions).toArray();

      assert.deepEqual(
        versions.map(it => ({ num: it.num, isHighestOfReleaseTrack: it.isHighestOfReleaseTrack })),
        [
          { num: '0.4.0-rc.1', isHighestOfReleaseTrack: false },
          { num: '0.3.24-alpha.02', isHighestOfReleaseTrack: false },
          { num: '0.3.23', isHighestOfReleaseTrack: true },
          { num: '0.3.22', isHighestOfReleaseTrack: false },
          { num: '0.3.21-pre.0', isHighestOfReleaseTrack: false },
          { num: '0.3.20', isHighestOfReleaseTrack: false },
          { num: '0.3.3', isHighestOfReleaseTrack: false },
          { num: '0.3.2', isHighestOfReleaseTrack: false },
          { num: '0.3.1', isHighestOfReleaseTrack: false },
          { num: '0.3.0-alpha.01', isHighestOfReleaseTrack: false },
          { num: '0.3.0', isHighestOfReleaseTrack: false },
          { num: '0.2.1', isHighestOfReleaseTrack: true },
          { num: '0.2.0', isHighestOfReleaseTrack: false },
          { num: '0.1.2', isHighestOfReleaseTrack: true },
          { num: '0.1.1', isHighestOfReleaseTrack: false },
        ],
      );
    });

    test('ignores yanked versions', async function (assert) {
      let crate = this.server.create('crate');
      this.server.create('version', { crate, num: '0.4.0' });
      this.server.create('version', { crate, num: '0.4.1' });
      this.server.create('version', { crate, num: '0.4.2', yanked: true });

      let crateRecord = await this.store.findRecord('crate', crate.name);
      let versions = (await crateRecord.versions).toArray();

      assert.deepEqual(
        versions.map(it => ({ num: it.num, isHighestOfReleaseTrack: it.isHighestOfReleaseTrack })),
        [
          { num: '0.4.0', isHighestOfReleaseTrack: false },
          { num: '0.4.1', isHighestOfReleaseTrack: true },
          { num: '0.4.2', isHighestOfReleaseTrack: true },
        ],
      );
    });
  });

  module('featuresList', function () {
    async function prepare(context, { features }) {
      let { server, store } = context;

      let crate = server.create('crate');
      server.create('version', { crate, features });

      let crateRecord = await store.findRecord('crate', crate.name);
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

    let crateRecord = await this.store.findRecord('crate', crate.name);
    assert.ok(crateRecord);
    let versions = (await crateRecord.versions).toArray();
    assert.equal(versions.length, 1);
    let version = versions[0];
    assert.ok(version.published_by);
    assert.equal(version.published_by.name, 'JD');
  });
});
