import ApplicationSerializer from './application';

export default class CrateOwnerInviteSerializer extends ApplicationSerializer {
  primaryKey = 'crate_id';

  modelNameFromPayloadKey(payloadKey) {
    if (payloadKey === 'users') return 'user';
    return 'crate-owner-invite';
  }

  payloadKeyFromModelName() {
    return 'crate_owner_invite';
  }

  keyForRelationship(key) {
    // Ember Data expects e.g. an `inviter` key in the payload, but the backend
    // uses `inviter_id` instead. This method makes sure that Ember Data can
    // find the correct relationship.
    return `${key}_id`;
  }
}
