import semverParse from 'semver/functions/parse.js';
import semverSort from 'semver/functions/rsort.js';

export function calculateReleaseTracks(versions: { yanked: boolean; num: string }[]) {
  let versionNums = versions.filter(it => !it.yanked).map(it => it.num);
  semverSort(versionNums, { loose: true });

  let tracks: Record<string, { highest: string }> = {};
  for (let num of versionNums) {
    let semver = semverParse(num, { loose: true });
    if (!semver || semver.prerelease.length !== 0) continue;
    let name = semver.major == 0 ? `0.${semver.minor}` : `${semver.major}`;
    if (name in tracks) continue;
    tracks[name] = { highest: num };
  }
  return tracks;
}
