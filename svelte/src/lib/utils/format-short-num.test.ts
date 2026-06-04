import { describe, expect, it } from 'vitest';

import { formatShortNum } from './format-short-num';

describe('formatShortNum()', () => {
  describe('formats numbers without suffix (below four digits)', () => {
    it.each([
      [0, '0'],
      [1, '1'],
      [500, '500'],
      [999, '999'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input)).toBe(expected);
    });
  });

  describe('formats numbers with K suffix', () => {
    it.each([
      [1000, '1.0K'],
      [1234, '1.2K'],
      [1500, '1.5K'],
      [10_000, '10K'],
      [50_000, '50K'],
      [100_000, '100K'],
      [500_000, '500K'],
      [999_499, '999K'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input)).toBe(expected);
    });
  });

  describe('formats numbers with M suffix', () => {
    it.each([
      [999_500, '1.0M'],
      [1_000_000, '1.0M'],
      [1_500_000, '1.5M'],
      [10_000_000, '10M'],
      [100_000_000, '100M'],
      [999_499_000, '999M'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input)).toBe(expected);
    });
  });

  describe('formats numbers with B suffix', () => {
    it.each([
      [999_500_000, '1.0B'],
      [1_000_000_000, '1.0B'],
      [1_743_000_000, '1.7B'],
      [1_000_000_000_000, '1,000B'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input)).toBe(expected);
    });
  });

  describe('matches the OpenGraph image threshold of 1500', () => {
    it.each([
      [1499, '1,499'],
      [1500, '1.5K'],
      [999_999, '1,000K'],
      [1_499_999, '1,500K'],
      [1_500_000, '1.5M'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input, { threshold: 1500 })).toBe(expected);
    });
  });
});
