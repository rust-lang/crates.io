import type { Advisory } from './rustsec';

import { describe, expect, it } from 'vitest';

import { versionRanges } from './rustsec';

describe('versionRanges', () => {
  it('returns null when advisory has no affected field', () => {
    let advisory = { id: 'X', summary: '', details: '' } satisfies Advisory;
    expect(versionRanges(advisory)).toBe(null);
  });

  it('returns null when advisory.affected is an empty array', () => {
    let advisory = { id: 'X', summary: '', details: '', affected: [] } satisfies Advisory;
    expect(versionRanges(advisory)).toBe(null);
  });

  it('returns null when affected has no ranges', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [{ ranges: [] }],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe(null);
  });

  it('returns null when ranges have no events', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [{ ranges: [{ type: 'SEMVER', events: [] }] }],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe(null);
  });

  it('ignores non-SEMVER range types', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [{ type: 'GIT', events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }] }],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe(null);
  });

  it('handles a simple range starting from 0.0.0-0', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [{ type: 'SEMVER', events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.5' }] }],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('<1.0.5');
  });

  it('handles a simple range with no introduced event', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [{ type: 'SEMVER', events: [{ fixed: '2.3.1' }] }],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('<2.3.1');
  });

  it('handles a range with explicit introduced version', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [{ type: 'SEMVER', events: [{ introduced: '0.8.0' }, { fixed: '0.8.13' }] }],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('>=0.8.0, <0.8.13');
  });

  it('handles multiple ranges in a single advisory', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [
            {
              type: 'SEMVER',
              events: [{ introduced: '0.0.0-0' }, { fixed: '0.7.46' }, { introduced: '0.8.0' }, { fixed: '0.8.13' }],
            },
          ],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('<0.7.46; >=0.8.0, <0.8.13');
  });

  it('handles multiple affected entries', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [{ type: 'SEMVER', events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }] }],
        },
        {
          ranges: [{ type: 'SEMVER', events: [{ introduced: '2.0.0' }, { fixed: '2.5.0' }] }],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('<1.0.0; >=2.0.0, <2.5.0');
  });

  it('handles multiple ranges within an affected entry', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [
            { type: 'SEMVER', events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }] },
            { type: 'SEMVER', events: [{ introduced: '2.0.0' }, { fixed: '2.5.0' }] },
          ],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('<1.0.0; >=2.0.0, <2.5.0');
  });

  it('skips ranges that only have introduced events', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [{ type: 'SEMVER', events: [{ introduced: '0.0.0-0' }] }],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe(null);
  });

  it('handles complex interleaved events', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [
            {
              type: 'SEMVER',
              events: [
                { introduced: '0.0.0-0' },
                { fixed: '0.7.46' },
                { introduced: '0.8.0' },
                { fixed: '0.8.13' },
                { introduced: '1.0.0' },
                { fixed: '1.2.0' },
              ],
            },
          ],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('<0.7.46; >=0.8.0, <0.8.13; >=1.0.0, <1.2.0');
  });

  it('filters out SEMVER ranges mixed with other types', () => {
    let advisory = {
      id: 'X',
      summary: '',
      details: '',
      affected: [
        {
          ranges: [
            { type: 'GIT', events: [{ introduced: 'abc123' }, { fixed: 'def456' }] },
            { type: 'SEMVER', events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }] },
          ],
        },
      ],
    } satisfies Advisory;
    expect(versionRanges(advisory)).toBe('<1.0.0');
  });
});
