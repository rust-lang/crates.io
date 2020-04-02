import BaseSerializer from './application';

export default BaseSerializer.extend({
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
    hash.id = Number(hash.id);
    if (hash.created_at) {
      hash.created_at = new Date(hash.created_at).toISOString();
    }
    if (hash.last_used_at) {
      hash.last_used_at = new Date(hash.last_used_at).toISOString();
    }
    delete hash.token;
    delete hash.user_id;
  },
});
