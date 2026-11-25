import semverParse from 'semver/functions/parse.js';

import { calculateDefaultVersion } from './default-version.js';

type Version = { yanked: boolean; num: string };

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

  let tracks: Record<string, { highest: string }> = {};
  for (let [name, trackVersions] of versionsByTrack) {
    tracks[name] = { highest: calculateDefaultVersion(trackVersions)!.num };
  }
  return tracks;
}
