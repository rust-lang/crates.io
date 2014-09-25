import {
  formatNum
} from 'cargo/helpers/format-num';

module('FormatNumHelper');

// Replace this with your real tests.
test('it works', function(assert) {
  assert.equal(formatNum(42), '42');
  assert.equal(formatNum(0), '0');
  assert.equal(formatNum(1000), '1,000');
});
