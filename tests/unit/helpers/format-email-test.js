import { module, test } from 'qunit';

import { formatEmail } from '../../../helpers/format-email';

module('Unit | Helper | format-email', function () {
  // Replace this with your real tests.
  test('it works', function (assert) {
    assert.equal(formatEmail('foo'), 'foo');
    assert.equal(formatEmail('foo <foo@bar.com>').toString(), `<a href='mailto:foo@bar.com'>foo</a>`);
    assert.equal(
      formatEmail('<script></script> <foo@bar.com>').toString(),
      `<a href='mailto:script&gt;&lt;/script&gt; &lt;foo@bar.com'></a>`,
    );
    assert.equal(formatEmail('').toString(), '');
    assert.equal(formatEmail('test <foo').toString(), 'test &lt;foo');
  });
});
