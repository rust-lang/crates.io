import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

module('Unit | Helper | format-num', function (hooks) {
  setupRenderingTest(hooks);

  test('it works', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    await render(hbs`{{format-num 42}}`);
    assert.dom().hasText('42');

    await render(hbs`{{format-num 0}}`);
    assert.dom().hasText('0');

    await render(hbs`{{format-num 0.2}}`);
    assert.dom().hasText('0.2');

    await render(hbs`{{format-num 1000}}`);
    assert.dom().hasText('1,000');

    await render(hbs`{{format-num 1000000}}`);
    assert.dom().hasText('1,000,000');
  });
});
