import { describe, expect, it } from 'vitest';

describe('__TEST__ constant', () => {
  it('is set to true when running in Vitest', () => {
    expect(__TEST__).toBe(true);
  });
});
