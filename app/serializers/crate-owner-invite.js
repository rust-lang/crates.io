import ApplicationSerializer from './application';

export default class CrateOwnerInviteSerializer extends ApplicationSerializer {
  primaryKey = 'crate_id';

  modelNameFromPayloadKey() {
    return 'crate-owner-invite';
  }

  payloadKeyFromModelName() {
    return 'crate_owner_invite';
  }
}
