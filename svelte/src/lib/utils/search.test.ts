import { describe, expect, test } from 'vitest';

import { hasMultiCategoryFilter, processSearchQuery } from './search';

describe('processSearchQuery', () => {
  test.for([
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
  ] as const)('%s', ([input, expected]) => {
    expect(processSearchQuery(input)).toEqual(expected);
  });
});

describe('hasMultiCategoryFilter', () => {
  test.for([
    ['foo bar', false],
    ['foo category:bar', false],
    ['foo category:bar category:baz', true],
    ['foo category:', false],
  ] as const)('%s → %s', ([input, expected]) => {
    expect(hasMultiCategoryFilter(input)).toBe(expected);
  });
});
