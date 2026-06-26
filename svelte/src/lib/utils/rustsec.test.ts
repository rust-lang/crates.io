import type { Advisory } from './rustsec';

import { describe, expect, it, vi } from 'vitest';

import { enrichAdvisories, fetchAdvisories, versionRanges } from './rustsec';

const UNMAINTAINED = 'RUSTSEC-2021-0139';

function unmaintainedAdvisory(overrides: Partial<Advisory> = {}): Advisory {
  return {
    id: UNMAINTAINED,
    summary: 'foo is unmaintained',
    details: '',
    affected: [{ ranges: [], database_specific: { informational: 'unmaintained' } }],
    ...overrides,
  };
}

describe('fetchAdvisories', () => {
  it('returns the parsed advisory list', async () => {
    let advisories = [unmaintainedAdvisory()];
    let fetch = vi.fn().mockResolvedValue(Response.json(advisories));

    expect(await fetchAdvisories(fetch, 'foo')).toEqual(advisories);
    expect(fetch).toHaveBeenCalledWith('https://rustsec.org/packages/foo.json');
  });

  it('returns an empty array on a 404 response', async () => {
    let fetch = vi.fn().mockResolvedValue(Response.json('not found', { status: 404 }));
    expect(await fetchAdvisories(fetch, 'foo')).toEqual([]);
  });

  it('throws on other non-OK responses', async () => {
    let fetch = vi.fn().mockResolvedValue(Response.json('boom', { status: 500 }));
    await expect(fetchAdvisories(fetch, 'foo')).rejects.toThrow('HTTP error! status: 500');
  });
});

describe('enrichAdvisories', () => {
  it('enriches advisories with version ranges and CVSS', () => {
    let advisory: Advisory = {
      id: 'RUSTSEC-2020-0001',
      summary: 'vulnerable',
      details: '',
      affected: [{ ranges: [{ type: 'SEMVER', events: [{ introduced: '0.0.0-0' }, { fixed: '1.2.0' }] }] }],
      severity: [
        { type: 'CVSS_V3', score: 'CVSS:3.1/AV:N' },
        { type: 'CVSS_V4', score: 'CVSS:4.0/AV:N' },
      ],
    };

    let [enriched] = enrichAdvisories([advisory]);
    expect(enriched.versionRanges).toBe('<1.2.0');
    expect(enriched.cvss).toBe('CVSS:4.0/AV:N');
  });

  it('filters out withdrawn and informational unmaintained advisories', () => {
    let advisories: Advisory[] = [
      unmaintainedAdvisory(),
      { id: 'RUSTSEC-2020-0002', summary: 'withdrawn', details: '', withdrawn: '2022-01-01T00:00:00Z' },
      { id: 'RUSTSEC-2020-0003', summary: 'real', details: '' },
    ];

    let result = enrichAdvisories(advisories);
    expect(result.map(a => a.id)).toEqual(['RUSTSEC-2020-0003']);
  });
});

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
