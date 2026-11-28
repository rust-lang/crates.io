import semverParse from 'semver/functions/parse.js';

import { compareDates } from './dates.js';
import { calculateDefaultVersion } from './default-version.js';

type Version = { yanked: boolean; num: string; created_at: string };

interface ReleaseTrack {
  highest: string;
  yanked: boolean;
  count: number;
  first_published: string;
  last_published: string;
}

/**
 * The highest version of a track is chosen like the default version of a
 * crate, see `calculateDefaultVersion()`.
 */
export function calculateReleaseTracks(versions: Version[]) {
  let versionsByTrack = new Map<string, Version[]>();
  for (let version of versions) {
    let semver = semverParse(version.num, { loose: true });
    if (!semver) continue;
    let name = semver.major == 0 ? `0.${semver.minor}` : `${semver.major}`;
    let trackVersions = versionsByTrack.get(name) ?? [];
    trackVersions.push(version);
    versionsByTrack.set(name, trackVersions);
  }

  let tracks: Record<string, ReleaseTrack> = {};
  for (let [name, trackVersions] of versionsByTrack) {
    let highest = calculateDefaultVersion(trackVersions)!;
    let dates = trackVersions.map(it => it.created_at).toSorted(compareDates);
    tracks[name] = {
      highest: highest.num,
      yanked: highest.yanked,
      count: trackVersions.length,
      first_published: dates[0]!,
      last_published: dates.at(-1)!,
    };
  }
  return tracks;
}
