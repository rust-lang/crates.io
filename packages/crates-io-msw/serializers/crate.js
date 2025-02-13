import prerelease from 'semver/functions/prerelease.js';
import semverSort from 'semver/functions/rsort.js';

import { db } from '../index.js';
import { compareDates } from '../utils/dates.js';
import { serializeModel } from '../utils/serializers.js';

export function serializeCrate(
  crate,
  { calculateVersions = true, includeCategories = false, includeKeywords = false, includeVersions = false } = {},
) {
  let versions = db.version.findMany({ where: { crate: { id: { equals: crate.id } } } });
  if (versions.length === 0) {
    throw new Error(`crate \`${crate.name}\` has no associated versions`);
  }

  let versionsByNum = Object.fromEntries(versions.map(it => [it.num, it]));
  let versionNums = Object.keys(versionsByNum);
  semverSort(versionNums, { loose: true });

  let serialized = serializeModel(crate);

  serialized.id = crate.name;

  serialized.default_version =
    versionNums.find(it => !prerelease(it, { loose: true }) && !versionsByNum[it].yanked) ??
    versionNums.find(it => !versionsByNum[it].yanked) ??
    versionNums[0];

  serialized.num_versions = versions.length;

  serialized.yanked = versionsByNum[serialized.default_version]?.yanked ?? false;

  serialized.links = {
    owner_user: `/api/v1/crates/${crate.name}/owner_user`,
    owner_team: `/api/v1/crates/${crate.name}/owner_team`,
    reverse_dependencies: `/api/v1/crates/${crate.name}/reverse_dependencies`,
    version_downloads: `/api/v1/crates/${crate.name}/downloads`,
    versions: `/api/v1/crates/${crate.name}/versions`,
  };

  if (calculateVersions) {
    let unyankedVersions = versionNums.filter(it => !versionsByNum[it].yanked);
    serialized.max_version = unyankedVersions[0] ?? '0.0.0';
    serialized.max_stable_version = unyankedVersions.find(it => !prerelease(it, { loose: true })) ?? null;

    let newestVersions = versions.filter(it => !it.yanked).sort((a, b) => compareDates(b.updated_at, a.updated_at));
    serialized.newest_version = newestVersions[0]?.num ?? '0.0.0';
  } else {
    serialized.max_version = '0.0.0';
    serialized.newest_version = '0.0.0';
    serialized.max_stable_version = null;
  }

  serialized.categories = includeCategories ? crate.categories.map(c => c.id) : null;
  serialized.keywords = includeKeywords ? crate.keywords.map(k => k.id) : null;
  serialized.versions = includeVersions ? versions.map(k => k.id) : null;

  delete serialized._extra_downloads;

  return serialized;
}

export function compare(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}
