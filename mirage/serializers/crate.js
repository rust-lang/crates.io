import { assert } from '@ember/debug';

import prerelease from 'semver/functions/prerelease';
import semverSort from 'semver/functions/rsort';

import { compareIsoDates } from '../route-handlers/-utils';
import BaseSerializer from './application';

export default BaseSerializer.extend({
  attrs: [
    'badges',
    'categories',
    'created_at',
    'description',
    'documentation',
    'downloads',
    'recent_downloads',
    'homepage',
    'id',
    'keywords',
    'links',
    'newest_version',
    'name',
    'repository',
    'updated_at',
    'versions',
  ],

  links(crate) {
    return {
      owner_user: `/api/v1/crates/${crate.name}/owner_user`,
      owner_team: `/api/v1/crates/${crate.name}/owner_team`,
      reverse_dependencies: `/api/v1/crates/${crate.name}/reverse_dependencies`,
      version_downloads: `/api/v1/crates/${crate.name}/downloads`,
      versions: `/api/v1/crates/${crate.name}/versions`,
    };
  },

  getHashForResource() {
    let [hash, addToIncludes] = BaseSerializer.prototype.getHashForResource.apply(this, arguments);

    if (Array.isArray(hash)) {
      for (let resource of hash) {
        this._adjust(resource);
      }
    } else {
      this._adjust(hash);
    }

    return [hash, addToIncludes];
  },

  _adjust(hash) {
    let versions = this.schema.versions.where({ crateId: hash.id });
    assert(`crate \`${hash.name}\` has no associated versions`, versions.length !== 0);
    versions = versions.filter(it => !it.yanked);

    let versionNums = versions.models.map(it => it.num);
    semverSort(versionNums, { loose: true });
    hash.max_version = versionNums[0] ?? '0.0.0';
    hash.max_stable_version = versionNums.find(it => !prerelease(it, { loose: true })) ?? null;

    let newestVersions = versions.models.sort((a, b) => compareIsoDates(b.updated_at, a.updated_at));
    hash.newest_version = newestVersions[0]?.num ?? '0.0.0';

    hash.id = hash.name;

    hash.categories = hash.category_ids;
    delete hash.category_ids;

    hash.keywords = hash.keyword_ids;
    delete hash.keyword_ids;

    hash.versions = hash.version_ids;
    delete hash.version_ids;

    delete hash.team_owner_ids;
    delete hash.user_owner_ids;
  },
});
