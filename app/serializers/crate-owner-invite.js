import ApplicationSerializer from './application';

export default class CrateOwnerInviteSerializer extends ApplicationSerializer {
  primaryKey = 'crate_id';

  modelNameFromPayloadKey() {
    return 'crate-owner-invite';
  }

  payloadKeyFromModelName() {
    return 'crate_owner_invite';
  }

  normalizeResponse(store, schema, payload, id, requestType) {
    if (payload.users) {
      delete payload.users;
    }

    return super.normalizeResponse(store, schema, payload, id, requestType);
  }
}
