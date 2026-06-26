interface RangeEvent {
  introduced?: string;
  fixed?: string;
}

interface Range {
  type: string;
  events: RangeEvent[];
}

interface Affected {
  ranges: Range[];
  database_specific?: {
    informational?: string;
  };
}

export interface Advisory {
  id: string;
  summary: string;
  details: string;
  aliases?: string[];
  withdrawn?: string;
  affected?: Affected[];
  severity?: { type: string; score: string }[];
}

/** A security advisory enriched with the data the crate security page renders. */
export interface EnrichedAdvisory extends Advisory {
  versionRanges: string | null;
  cvss: string | null;
}

/**
 * Extracts version ranges from a RustSec advisory.
 *
 * OSV interleaves `introduced` and `fixed` events to represent ranges, like so:
 *
 * ```
 * "events": [
 *   { introduced: "0.0.0-0" },
 *   { fixed: "0.7.46" },
 *   { introduced: "0.8.0" },
 *   { fixed: "0.8.13" }
 * ]
 * ```
 */
export function versionRanges(advisory: Advisory): string | null {
  if (!advisory.affected || advisory.affected.length === 0) {
    return null;
  }

  let ranges: string[] = [];
  for (let affected of advisory.affected) {
    if (affected.ranges.length === 0) {
      continue;
    }

    for (let range of affected.ranges) {
      if (range.type !== 'SEMVER' || range.events.length === 0) {
        continue;
      }

      let currentIntroduced: string | null = null;
      for (let event of range.events) {
        if (event.introduced !== undefined) {
          currentIntroduced = event.introduced;
        } else if (event.fixed !== undefined) {
          if (currentIntroduced === null || currentIntroduced === '0.0.0-0') {
            ranges.push(`<${event.fixed}`);
          } else {
            let start = currentIntroduced === '0.0.0-0' ? '0' : currentIntroduced;
            ranges.push(`>=${start}, <${event.fixed}`);
            currentIntroduced = null;
          }
        }
      }
    }
  }

  return ranges.length === 0 ? null : ranges.join('; ');
}

function extractCvss(advisory: Advisory): string | null {
  let cvssEntry =
    advisory.severity?.find(s => s.type === 'CVSS_V4') ?? advisory.severity?.find(s => s.type === 'CVSS_V3');
  return cvssEntry?.score ?? null;
}

/**
 * Fetches the raw RustSec advisory list for a crate.
 *
 * Returns an empty array when the crate has no advisories (RustSec answers with
 * `404` in that case) and throws for any other non-OK response.
 */
export async function fetchAdvisories(fetch: typeof globalThis.fetch, crateId: string): Promise<Advisory[]> {
  let response = await fetch(`https://rustsec.org/packages/${crateId}.json`);
  if (response.status === 404) {
    return [];
  } else if (response.ok) {
    return response.json();
  } else {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
}

/**
 * Filters a raw advisory list down to the advisories shown on the crate security
 * page: withdrawn and purely informational `unmaintained` advisories are dropped,
 * and each remaining advisory is enriched with its affected version ranges and
 * CVSS score.
 */
export function enrichAdvisories(advisories: Advisory[]): EnrichedAdvisory[] {
  return advisories
    .filter(
      advisory =>
        !advisory.withdrawn &&
        !advisory.affected?.some(affected => affected.database_specific?.informational === 'unmaintained'),
    )
    .map(advisory => ({
      ...advisory,
      versionRanges: versionRanges(advisory),
      cvss: extractCvss(advisory),
    }));
}
