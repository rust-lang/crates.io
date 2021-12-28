import { module, test } from 'qunit';

import { processSearchQuery } from '../../utils/search';

module('processSearchQuery()', function () {
  const TESTS = [
    ['foo', { q: 'foo' }],
    ['  foo    bar     ', { q: 'foo bar' }],
    ['foo keywords:bar', { q: 'foo', keyword: 'bar' }],
    ['foo keywords:', { q: 'foo' }],
    ['keywords:bar foo', { q: 'foo', keyword: 'bar' }],
    ['foo \t   keywords:bar    baz', { q: 'foo baz', keyword: 'bar' }],
    ['foo keywords:bar,baz', { q: 'foo', all_keywords: 'bar baz' }],
    ['foo keywords:bar keywords:baz', { q: 'foo', keyword: 'baz' }],
    ['foo keyword:bar keyword:baz', { q: 'foo', all_keywords: 'bar baz' }],
  ];

  for (let [input, expectation] of TESTS) {
    test(input, function (assert) {
      assert.deepEqual(processSearchQuery(input), expectation);
    });
  }
});
