import { render } from '@ember/test-helpers';
import { setupRenderingTest } from 'ember-qunit';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

module('Unit | Helper | format-num', function (hooks) {
  setupRenderingTest(hooks);
  setupWindowMock(hooks);

  test('it works', async function (assert) {
    window.navigator = { language: 'en' };

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
