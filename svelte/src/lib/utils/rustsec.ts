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

/** The data the unmaintained banner needs about a single advisory. */
export interface Unmaintained {
  /** The RustSec advisory id, e.g. `RUSTSEC-2021-0139`. */
  id: string;
  /** Link to the advisory page on rustsec.org. */
  url: string;
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

/**
 * Fetches the raw RustSec advisory list for a crate.
 *
 * Returns an empty array when the crate has no advisories (RustSec answers with
 * `404` in that case) and throws for any other non-OK response.
 */
export async function fetchAdvisories(fetch: typeof globalThis.fetch, crateName: string): Promise<Advisory[]> {
  let response = await fetch(`https://rustsec.org/packages/${crateName}.json`);
  if (response.status === 404) {
    return [];
  } else if (response.ok) {
    return response.json();
  } else {
    throw new Error(`HTTP error! status: ${response.status}`);
  }
}

function extractCvss(advisory: Advisory): string | null {
  let cvssEntry =
    advisory.severity?.find(s => s.type === 'CVSS_V4') ?? advisory.severity?.find(s => s.type === 'CVSS_V3');
  return cvssEntry?.score ?? null;
}

/**
 * Loads the security advisories shown on the crate security page: withdrawn and
 * purely informational `unmaintained` advisories are filtered out, and each
 * remaining advisory is enriched with its affected version ranges and CVSS score.
 */
export async function loadAdvisories(fetch: typeof globalThis.fetch, crateName: string): Promise<EnrichedAdvisory[]> {
  let advisories = await fetchAdvisories(fetch, crateName);
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

/**
 * Returns `true` if the advisory marks the crate as unmaintained without offering
 * a way out: it must carry the `unmaintained` informational marker, must not have
 * been withdrawn, and must not point at any patched version.
 */
function isUnmaintained(advisory: Advisory): boolean {
  if (advisory.withdrawn) {
    return false;
  }

  let affected = advisory.affected ?? [];
  if (!affected.some(entry => entry.database_specific?.informational === 'unmaintained')) {
    return false;
  }

  let patched = affected.some(entry => entry.ranges?.some(range => range.events?.some(event => event.fixed != null)));
  return !patched;
}

/**
 * Fetches the RustSec advisory list for a crate and returns the first advisory
 * that marks it as unmaintained, or `null` if there is none.
 *
 * Failures (including the common `404` for crates without advisories) resolve to
 * `null` so a missing or unreachable RustSec database never breaks the page.
 */
export async function loadUnmaintained(
  fetch: typeof globalThis.fetch,
  crateName: string,
): Promise<Unmaintained | null> {
  let advisories: Advisory[];
  try {
    advisories = await fetchAdvisories(fetch, crateName);
  } catch {
    return null;
  }

  let advisory = advisories.find(isUnmaintained);
  if (!advisory) {
    return null;
  }

  return { id: advisory.id, url: `https://rustsec.org/advisories/${advisory.id}.html` };
}
