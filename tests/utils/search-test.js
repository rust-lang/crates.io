import { module, test } from 'qunit';

import { processSearchQuery } from '../../utils/search';

module('processSearchQuery()', function () {
  const TESTS = [
    ['foo', { q: 'foo' }],
    ['  foo    bar     ', { q: 'foo bar' }],
    ['foo keyword:bar', { q: 'foo', keyword: 'bar' }],
    ['foo keyword:', { q: 'foo' }],
    ['keyword:bar foo', { q: 'foo', keyword: 'bar' }],
    ['foo \t   keyword:bar    baz', { q: 'foo baz', keyword: 'bar' }],
    ['foo keyword:bar keyword:baz', { q: 'foo', all_keywords: 'bar baz' }],
    ['foo category:', { q: 'foo' }],
    ['foo category:no-std', { q: 'foo', category: 'no-std' }],
    ['foo category:no-std keyword:bar keyword:baz', { q: 'foo', all_keywords: 'bar baz', category: 'no-std' }],
  ];

  for (let [input, expectation] of TESTS) {
    test(input, function (assert) {
      assert.deepEqual(processSearchQuery(input), expectation);
    });
  }
});
