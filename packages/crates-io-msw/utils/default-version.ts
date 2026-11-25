import semverCompareBuild from 'semver/functions/compare-build.js';
import prerelease from 'semver/functions/prerelease.js';

/**
 * Returns the highest stable non-yanked version, then the highest non-yanked
 * version, then the highest version.
 */
export function calculateDefaultVersion<T extends { num: string; yanked: boolean }>(versions: T[]): T | undefined {
  let sorted = versions.toSorted((a, b) => semverCompareBuild(b.num, a.num, { loose: true }));
  return (
    sorted.find(it => !it.yanked && !prerelease(it.num, { loose: true })) ?? sorted.find(it => !it.yanked) ?? sorted[0]
  );
}
