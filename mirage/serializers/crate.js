import { assert } from '@ember/debug';
import semverSort from 'semver/functions/rsort';
import BaseSerializer from './application';
import { compareIsoDates } from '../route-handlers/-utils';

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
      owner_user: `/api/v1/crates/${crate.id}/owner_user`,
      owner_team: `/api/v1/crates/${crate.id}/owner_team`,
      reverse_dependencies: `/api/v1/crates/${crate.id}/reverse_dependencies`,
      version_downloads: `/api/v1/crates/${crate.id}/downloads`,
      versions: `/api/v1/crates/${crate.id}/versions`,
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
    assert(`crate \`${hash.id}\` has no associated versions`, versions.length !== 0);

    let versionNums = versions.models.map(it => it.num);
    semverSort(versionNums);
    hash.max_version = versionNums[0];

    let newestVersions = versions.sort((a, b) => compareIsoDates(b.updated_at, a.updated_at));
    hash.newest_version = newestVersions.models[0].num;

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
