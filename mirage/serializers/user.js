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
      delete hash.email_verification_sent;
    }
  },
});
