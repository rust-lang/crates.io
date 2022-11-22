import { module, test } from 'qunit';

import { formatEmail } from '../../../helpers/format-email';

module('Unit | Helper | format-email', function () {
  // Replace this with your real tests.
  test('it works', function (assert) {
    assert.strictEqual(formatEmail('foo').toString(), 'foo');
    assert.strictEqual(formatEmail('foo <foo@bar.com>').toString(), `<a href='mailto:foo@bar.com'>foo</a>`);
    assert.strictEqual(
      formatEmail('<script></script> <foo@bar.com>').toString(),
      `<a href='mailto:script&gt;&lt;/script&gt; &lt;foo@bar.com'></a>`,
    );
    assert.strictEqual(formatEmail('').toString(), '');
    assert.strictEqual(formatEmail('test <foo').toString(), 'test &lt;foo');
  });
});
