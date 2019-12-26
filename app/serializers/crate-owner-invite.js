import ApplicationSerializer from './application';

export default ApplicationSerializer.extend({
  primaryKey: 'crate_id',
  modelNameFromPayloadKey() {
    return 'crate-owner-invite';
  },
  payloadKeyFromModelName() {
    return 'crate_owner_invite';
  },
});
