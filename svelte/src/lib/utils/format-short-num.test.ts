import { describe, expect, it } from 'vitest';

import { formatShortNum } from './format-short-num';

describe('formatShortNum()', () => {
  describe('formats numbers without suffix (below 1500)', () => {
    it.each([
      [0, '0'],
      [1, '1'],
      [1000, '1,000'],
      [1499, '1,499'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input)).toBe(expected);
    });
  });

  describe('formats numbers with K suffix (1500 to 1500000)', () => {
    it.each([
      [1500, '1.5K'],
      [2000, '2.0K'],
      [5000, '5.0K'],
      [10_000, '10K'],
      [50_000, '50K'],
      [100_000, '100K'],
      [500_000, '500K'],
      [999_999, '1,000K'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input)).toBe(expected);
    });
  });

  describe('formats numbers with M suffix (above 1500000)', () => {
    it.each([
      [1_500_000, '1.5M'],
      [2_000_000, '2.0M'],
      [5_000_000, '5.0M'],
      [10_000_000, '10M'],
      [50_000_000, '50M'],
      [100_000_000, '100M'],
      [1_000_000_000, '1,000M'],
    ])('%d → %s', (input, expected) => {
      expect(formatShortNum(input)).toBe(expected);
    });
  });
});
