import BaseSerializer from './application';

export default BaseSerializer.extend({
  // eslint-disable-next-line ember/avoid-leaking-state-in-ember-objects
  include: ['inviter', 'invitee'],

  getHashForResource() {
    let [hash, addToIncludes] = BaseSerializer.prototype.getHashForResource.apply(this, arguments);

    if (Array.isArray(hash)) {
      for (let resource of hash) {
        this._adjust(resource);
      }
    } else {
      this._adjust(hash);
    }

    addToIncludes.sort((a, b) => a.id - b.id);

    return [hash, addToIncludes];
  },

  _adjust(hash) {
    delete hash.id;
    delete hash.token;

    hash.crate_id = Number(hash.crate_id);

    let crate = this.schema.crates.find(hash.crate_id);
    hash.crate_name = crate.name;

    hash.invitee_id = Number(hash.invitee_id);
    hash.inviter_id = Number(hash.inviter_id);

    let inviter = this.schema.users.find(hash.inviter_id);
    hash.invited_by_username = inviter.login;
  },
});
