import { module, test } from 'qunit';

import { calculateReleaseTracks } from '@crates-io/msw/utils/release-tracks';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Model | Version', function (hooks) {
  setupTest(hooks);
  setupMsw(hooks);
  setupWindowMock(hooks);

  hooks.beforeEach(function () {
    this.store = this.owner.lookup('service:store');
  });

  test('isNew', async function (assert) {
    let { db, store } = this;

    let crate = db.crate.create();
    db.version.create({ crate, created_at: '2010-06-16T21:30:45Z' });

    let crateRecord = await store.findRecord('crate', crate.name);
    let versions = (await crateRecord.versions).slice();

    this.clock.setSystemTime(new Date('2010-06-16T21:40:45Z'));
    assert.true(versions[0].isNew);

    this.clock.setSystemTime(new Date('2010-06-23T21:40:45Z'));
    assert.true(versions[0].isNew);

    this.clock.setSystemTime(new Date('2010-06-24T21:40:45Z'));
    assert.false(versions[0].isNew);

    this.clock.setSystemTime(new Date('2014-06-24T21:40:45Z'));
    assert.false(versions[0].isNew);
  });

  test('editionMsrv', async function (assert) {
    let version = await this.store.createRecord('version');
    assert.strictEqual(version.editionMsrv, undefined);

    version.edition = '2015';
    assert.strictEqual(version.editionMsrv, undefined);

    version.edition = '2018';
    assert.strictEqual(version.editionMsrv, '1.31.0');

    version.edition = '2021';
    assert.strictEqual(version.editionMsrv, '1.56.0');

    version.edition = '2024';
    assert.strictEqual(version.editionMsrv, '1.85.0');

    version.edition = '2027';
    assert.strictEqual(version.editionMsrv, undefined);
  });

  test('msrv', async function (assert) {
    let version = await this.store.createRecord('version');
    assert.strictEqual(version.msrv, undefined);

    version.rust_version = '1.69.1';
    assert.strictEqual(version.msrv, '1.69.1');

    version.rust_version = '1.69';
    assert.strictEqual(version.msrv, '1.69.0');

    // this is not actually allowed by the backend
    version.rust_version = '1';
    assert.strictEqual(version.msrv, '1');
  });

  module('semver', function () {
    async function prepare(context, { num }) {
      let { db, store } = context;

      let crate = db.crate.create();
      db.version.create({ crate, num });

      let crateRecord = await store.findRecord('crate', crate.name);
      let versions = (await crateRecord.versions).slice();
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
        '0.3.0',
        '0.3.0-alpha.01',
        '0.2.1',
        '0.2.0',
        '0.1.2',
        '0.1.1',
      ];

      let crate = this.db.crate.create();
      for (let num of nums.toReversed()) {
        this.db.version.create({ crate, num });
      }

      let crateRecord = await this.store.findRecord('crate', crate.name);
      let versions = (await crateRecord.loadVersionsTask.perform()).slice();
      crateRecord.release_tracks = calculateReleaseTracks(versions);

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
          { num: '0.3.0', isHighestOfReleaseTrack: false },
          { num: '0.3.0-alpha.01', isHighestOfReleaseTrack: false },
          { num: '0.2.1', isHighestOfReleaseTrack: true },
          { num: '0.2.0', isHighestOfReleaseTrack: false },
          { num: '0.1.2', isHighestOfReleaseTrack: true },
          { num: '0.1.1', isHighestOfReleaseTrack: false },
        ],
      );
    });

    test('ignores yanked versions', async function (assert) {
      let crate = this.db.crate.create();
      this.db.version.create({ crate, num: '0.4.0' });
      this.db.version.create({ crate, num: '0.4.1' });
      this.db.version.create({ crate, num: '0.4.2', yanked: true });

      let crateRecord = await this.store.findRecord('crate', crate.name);
      let versions = (await crateRecord.loadVersionsTask.perform()).slice();
      crateRecord.release_tracks = calculateReleaseTracks(versions);

      assert.deepEqual(
        versions.map(it => ({ num: it.num, isHighestOfReleaseTrack: it.isHighestOfReleaseTrack })),
        [
          { num: '0.4.2', isHighestOfReleaseTrack: false },
          { num: '0.4.1', isHighestOfReleaseTrack: true },
          { num: '0.4.0', isHighestOfReleaseTrack: false },
        ],
      );
    });

    test('handles newly released versions correctly', async function (assert) {
      let crate = this.db.crate.create();
      this.db.version.create({ crate, num: '0.4.0' });
      this.db.version.create({ crate, num: '0.4.1' });

      let crateRecord = await this.store.findRecord('crate', crate.name);
      let versions = (await crateRecord.loadVersionsTask.perform()).slice();
      crateRecord.release_tracks = calculateReleaseTracks(versions);

      assert.deepEqual(
        versions.map(it => ({ num: it.num, isHighestOfReleaseTrack: it.isHighestOfReleaseTrack })),
        [
          { num: '0.4.1', isHighestOfReleaseTrack: true },
          { num: '0.4.0', isHighestOfReleaseTrack: false },
        ],
      );

      this.db.version.create({ crate, num: '0.4.2' });
      this.db.version.create({ crate, num: '0.4.3', yanked: true });
      crateRecord = await this.store.findRecord('crate', crate.name, { reload: true });
      versions = (await crateRecord.loadVersionsTask.perform({ reload: true })).slice();
      crateRecord.release_tracks = calculateReleaseTracks(versions);

      assert.deepEqual(
        versions.map(it => ({ num: it.num, isHighestOfReleaseTrack: it.isHighestOfReleaseTrack })),
        [
          { num: '0.4.3', isHighestOfReleaseTrack: false },
          { num: '0.4.2', isHighestOfReleaseTrack: true },
          { num: '0.4.1', isHighestOfReleaseTrack: false },
          { num: '0.4.0', isHighestOfReleaseTrack: false },
        ],
      );
    });
  });

  module('featuresList', function () {
    async function prepare(context, { features }) {
      let { db, store } = context;

      let crate = db.crate.create();
      db.version.create({ crate, features });

      let crateRecord = await store.findRecord('crate', crate.name);
      let versions = (await crateRecord.versions).slice();
      return versions[0];
    }

    test('`features: {}` results in empty list', async function (assert) {
      let version = await prepare(this, { features: {} });
      assert.deepEqual(version.featureList, []);
    });

    test('`features: undefined` results in empty list', async function (assert) {
      let version = await prepare(this, { features: undefined });
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
    let user = this.db.user.create({ name: 'JD' });

    let crate = this.db.crate.create();
    this.db.version.create({ crate, publishedBy: user });

    let crateRecord = await this.store.findRecord('crate', crate.name);
    assert.ok(crateRecord);
    let versions = (await crateRecord.versions).slice();
    assert.strictEqual(versions.length, 1);
    let version = versions[0];
    assert.ok(version.published_by);
    assert.strictEqual(version.published_by.name, 'JD');
  });

  module('purl', function () {
    test('generates PURL for crates.io version', async function (assert) {
      let { db, store } = this;

      window.location = 'https://crates.io';

      let crate = db.crate.create({ name: 'serde' });
      db.version.create({ crate, num: '1.0.136' });

      let crateRecord = await store.findRecord('crate', crate.name);
      let versions = (await crateRecord.versions).slice();
      let version = versions[0];

      assert.strictEqual(version.purl, 'pkg:cargo/serde@1.0.136');
    });

    test('generates PURL with registry URL for non-crates.io hosts', async function (assert) {
      let { db, store } = this;

      window.location = 'https://staging.crates.io';

      let crate = db.crate.create({ name: 'test-crate' });
      db.version.create({ crate, num: '2.5.0' });

      let crateRecord = await store.findRecord('crate', crate.name);
      let versions = (await crateRecord.versions).slice();
      let version = versions[0];

      assert.strictEqual(version.purl, 'pkg:cargo/test-crate@2.5.0?repository_url=https%3A%2F%2Fstaging.crates.io%2F');
    });
  });
});
