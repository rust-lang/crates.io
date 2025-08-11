import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import formatNum from 'crates-io/helpers/format-num';
import { setupRenderingTest } from 'crates-io/tests/helpers';

module('Unit | Helper | format-num', function (hooks) {
  setupRenderingTest(hooks);

  test('it works', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await render(<template>{{formatNum 42}}</template>);
    assert.dom().hasText('42');

    await render(<template>{{formatNum 0}}</template>);
    assert.dom().hasText('0');

    await render(<template>{{formatNum 0.2}}</template>);
    assert.dom().hasText('0.2');

    await render(<template>{{formatNum 1000}}</template>);
    assert.dom().hasText('1,000');

    await render(<template>{{formatNum 1000000}}</template>);
    assert.dom().hasText('1,000,000');
  });
});
