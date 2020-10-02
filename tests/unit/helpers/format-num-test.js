import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { formatNum } from '../../../helpers/format-num';

module('Unit | Helper | format-num', function (hooks) {
  setupWindowMock(hooks);

  test('it works', function (assert) {
    window.navigator = { language: 'en' };

    assert.equal(formatNum(42), '42');
    assert.equal(formatNum(0), '0');
    assert.equal(formatNum(0.2), '0.2');
    assert.equal(formatNum(1000), '1,000');
    assert.equal(formatNum(1000000), '1,000,000');
  });
});
