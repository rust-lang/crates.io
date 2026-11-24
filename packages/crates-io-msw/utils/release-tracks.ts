import semverParse from 'semver/functions/parse.js';
import semverPrerelease from 'semver/functions/prerelease.js';
import semverSort from 'semver/functions/rsort.js';

export function calculateReleaseTracks(versions: { yanked: boolean; num: string }[]) {
  let versionNums = versions.filter(it => !it.yanked).map(it => it.num);
  semverSort(versionNums, { loose: true });

  let tracks: Record<string, { highest: string }> = {};
  for (let num of versionNums) {
    let semver = semverParse(num, { loose: true });
    if (!semver) continue;
    let name = semver.major == 0 ? `0.${semver.minor}` : `${semver.major}`;
    let track = tracks[name];
    // Stable versions take precedence over higher pre-releases.
    if (!track || (semverPrerelease(track.highest, { loose: true }) && semver.prerelease.length === 0)) {
      tracks[name] = { highest: num };
    }
  }
  return tracks;
}
