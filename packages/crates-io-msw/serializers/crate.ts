import type { components } from '@crates-io/api-client';
import type { Crate } from '../models/index.js';

import prerelease from 'semver/functions/prerelease.js';
import semverSort from 'semver/functions/rsort.js';

import { db } from '../index.js';
import { compareDates } from '../utils/dates.js';

type ApiCrate = components['schemas']['Crate'];

export function serializeCrate(
  crate: Crate,
  { calculateVersions = true, includeCategories = false, includeKeywords = false, includeVersions = false } = {},
): ApiCrate {
  let versions = db.version.findMany(q => q.where({ crate: { id: crate.id } }));
  if (versions.length === 0) {
    throw new Error(`crate \`${crate.name}\` has no associated versions`);
  }

  let versionsByNum = Object.fromEntries(versions.map(it => [it.num, it]));
  let versionNums = Object.keys(versionsByNum);
  semverSort(versionNums, { loose: true });

  let defaultVersion =
    versionNums.find(it => !prerelease(it, { loose: true }) && !versionsByNum[it].yanked) ??
    versionNums.find(it => !versionsByNum[it].yanked) ??
    versionNums[0];
  let maxVersion = '0.0.0';
  let newestVersion = '0.0.0';
  let maxStableVersion = null;

  if (calculateVersions) {
    let unyankedVersions = versionNums.filter(it => !versionsByNum[it].yanked);
    maxVersion = unyankedVersions[0] ?? '0.0.0';
    maxStableVersion = unyankedVersions.find(it => !prerelease(it, { loose: true })) ?? null;

    let newestVersions = versions.filter(it => !it.yanked).toSorted((a, b) => compareDates(b.updated_at, a.updated_at));
    newestVersion = newestVersions[0]?.num ?? '0.0.0';
  }

  return {
    id: crate.name,
    name: crate.name,
    description: crate.description,
    downloads: crate.downloads,
    recent_downloads: crate.recent_downloads,
    documentation: crate.documentation,
    homepage: crate.homepage,
    repository: crate.repository,
    created_at: crate.created_at,
    updated_at: crate.updated_at,
    badges: crate.badges,
    trustpub_only: crate.trustpubOnly,
    exact_match: false,
    default_version: defaultVersion,
    num_versions: versions.length,
    yanked: versionsByNum[defaultVersion]?.yanked ?? false,
    links: {
      owners: `/api/v1/crates/${crate.name}/owners`,
      owner_user: `/api/v1/crates/${crate.name}/owner_user`,
      owner_team: `/api/v1/crates/${crate.name}/owner_team`,
      reverse_dependencies: `/api/v1/crates/${crate.name}/reverse_dependencies`,
      version_downloads: `/api/v1/crates/${crate.name}/downloads`,
      versions: `/api/v1/crates/${crate.name}/versions`,
    },
    max_version: maxVersion,
    newest_version: newestVersion,
    max_stable_version: maxStableVersion,
    categories: includeCategories ? crate.categories.map(c => c.id) : null,
    keywords: includeKeywords ? crate.keywords.map(k => k.id) : null,
    versions: includeVersions ? versions.map(k => k.id) : null,
  };
}

export function compare(a: string, b: string) {
  return a < b ? -1 : a > b ? 1 : 0;
}
