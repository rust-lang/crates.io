import { module, test } from 'qunit';

import { versionRanges } from 'crates-io/utils/version-ranges';

module('Utils | version-ranges', function () {
  module('versionRanges()', function () {
    test('returns null when advisory has no affected field', function (assert) {
      let advisory = {};
      let result = versionRanges(advisory);
      assert.strictEqual(result, null);
    });

    test('returns null when advisory.affected is an empty array', function (assert) {
      let advisory = { affected: [] };
      let result = versionRanges(advisory);
      assert.strictEqual(result, null);
    });

    test('returns null when affected has no ranges', function (assert) {
      let advisory = {
        affected: [{ ranges: [] }],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, null);
    });

    test('returns null when ranges have no events', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [{ type: 'SEMVER', events: [] }],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, null);
    });

    test('ignores non-SEMVER range types', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'GIT',
                events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, null);
    });

    test('handles a simple range starting from 0.0.0-0', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.5' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '<1.0.5');
    });

    test('handles a simple range with no introduced event', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ fixed: '2.3.1' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '<2.3.1');
    });

    test('handles a range with explicit introduced version', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '0.8.0' }, { fixed: '0.8.13' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '>=0.8.0, <0.8.13');
    });

    test('handles multiple ranges in a single advisory', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '0.0.0-0' }, { fixed: '0.7.46' }, { introduced: '0.8.0' }, { fixed: '0.8.13' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '<0.7.46; >=0.8.0, <0.8.13');
    });

    test('handles multiple affected entries', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }],
              },
            ],
          },
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '2.0.0' }, { fixed: '2.5.0' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '<1.0.0; >=2.0.0, <2.5.0');
    });

    test('handles multiple ranges within an affected entry', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }],
              },
              {
                type: 'SEMVER',
                events: [{ introduced: '2.0.0' }, { fixed: '2.5.0' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '<1.0.0; >=2.0.0, <2.5.0');
    });

    test('skips ranges that only have introduced events', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '0.0.0-0' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, null);
    });

    test('handles complex interleaved events', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [
                  { introduced: '0.0.0-0' },
                  { fixed: '0.7.46' },
                  { introduced: '0.8.0' },
                  { fixed: '0.8.13' },
                  { introduced: '1.0.0' },
                  { fixed: '1.2.0' },
                ],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '<0.7.46; >=0.8.0, <0.8.13; >=1.0.0, <1.2.0');
    });

    test('filters out SEMVER ranges mixed with other types', function (assert) {
      let advisory = {
        affected: [
          {
            ranges: [
              {
                type: 'GIT',
                events: [{ introduced: 'abc123' }, { fixed: 'def456' }],
              },
              {
                type: 'SEMVER',
                events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }],
              },
            ],
          },
        ],
      };
      let result = versionRanges(advisory);
      assert.strictEqual(result, '<1.0.0');
    });
  });
});
