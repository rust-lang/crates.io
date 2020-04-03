import BaseSerializer from './application';

export default BaseSerializer.extend({
  getHashForResource() {
    let [hash, addToIncludes] = BaseSerializer.prototype.getHashForResource.apply(this, arguments);

    let removePrivateData = this.request.url !== '/api/v1/me';

    if (Array.isArray(hash)) {
      for (let resource of hash) {
        this._adjust(resource, { removePrivateData });
      }
    } else {
      this._adjust(hash, { removePrivateData });
    }

    return [hash, addToIncludes];
  },

  _adjust(hash, { removePrivateData }) {
    hash.id = Number(hash.id);

    if (removePrivateData) {
      delete hash.email;
      delete hash.email_verified;
    } else {
      hash.email_verification_sent = hash.email_verified || Boolean(hash.email_verification_token);
    }

    delete hash.email_verification_token;
    delete hash.followed_crate_ids;
  },
});
