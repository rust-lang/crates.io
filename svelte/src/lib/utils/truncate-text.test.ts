import { describe, expect, it } from 'vitest';

import { truncateText } from './truncate-text';

describe('truncateText', () => {
  it('returns text unchanged when shorter than maxLength', () => {
    expect(truncateText('hello')).toBe('hello');
    expect(truncateText('short text', 200)).toBe('short text');
  });

  it('returns text unchanged when exactly maxLength', () => {
    let text = 'a'.repeat(200);
    expect(truncateText(text)).toBe(text);
  });

  it('truncates text longer than maxLength and appends ellipsis', () => {
    let text = 'a'.repeat(201);
    expect(truncateText(text)).toBe('a'.repeat(200) + ' …');
  });

  it('respects custom maxLength', () => {
    expect(truncateText('hello world', 5)).toBe('hello …');
    expect(truncateText('hello', 5)).toBe('hello');
    expect(truncateText('hello!', 5)).toBe('hello …');
  });

  it('handles empty string', () => {
    expect(truncateText('')).toBe('');
  });

  it('truncates and trims trailing whitespace', () => {
    let text = 'The quick brown fox jumps over the lazy dog';
    expect(truncateText(text, 10)).toBe('The quick …');
  });
});
