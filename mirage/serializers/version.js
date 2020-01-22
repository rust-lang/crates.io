import BaseSerializer from './application';

export default BaseSerializer.extend({
  attrs: [
    'crate_id',
    'created_at',
    'downloads',
    'features',
    'id',
    'links',
    'num',
    'updated_at',
    'yanked',
    'license',
    'crate_size',
  ],

  links(version) {
    return {
      authors: `/api/v1/crates/${version.crateId}/${version.num}/authors`,
      dependencies: `/api/v1/crates/${version.crateId}/${version.num}/dependencies`,
      version_downloads: `/api/v1/crates/${version.crateId}/${version.num}/downloads`,
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
    hash.dl_path = `/api/v1/crates/${hash.crate_id}/${hash.num}/download`;
    hash.crate = hash.crate_id;
    delete hash.crate_id;
  },
});
