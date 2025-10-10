import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import formatShortNum from 'crates-io/helpers/format-short-num';
import { setupRenderingTest } from 'crates-io/tests/helpers';

module('Unit | Helper | format-short-num', function (hooks) {
  setupRenderingTest(hooks);

  async function check(assert, input, expected) {
    await render(<template>{{formatShortNum input}}</template>);
    assert.dom().hasText(expected);
  }

  test('formats numbers without suffix (below 1500)', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await check(assert, 0, '0');
    await check(assert, 1, '1');
    await check(assert, 1000, '1,000');
    await check(assert, 1499, '1,499');
  });

  test('formats numbers with K suffix (1500 to 1500000)', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await check(assert, 1500, '1.5K');
    await check(assert, 2000, '2.0K');
    await check(assert, 5000, '5.0K');
    await check(assert, 10_000, '10K');
    await check(assert, 50_000, '50K');
    await check(assert, 100_000, '100K');
    await check(assert, 500_000, '500K');
    await check(assert, 999_999, '1,000K');
  });

  test('formats numbers with M suffix (above 1500000)', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await check(assert, 1_500_000, '1.5M');
    await check(assert, 2_000_000, '2.0M');
    await check(assert, 5_000_000, '5.0M');
    await check(assert, 10_000_000, '10M');
    await check(assert, 50_000_000, '50M');
    await check(assert, 100_000_000, '100M');
    await check(assert, 1_000_000_000, '1,000M');
  });
});
