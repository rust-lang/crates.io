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
    'max_version',
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
