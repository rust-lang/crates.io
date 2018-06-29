import DS from 'ember-data';

export default DS.RESTSerializer.extend({
    primaryKey: 'crate_id',
    modelNameFromPayloadKey() {
        return 'crate-owner-invite';
    },
    payloadKeyFromModelName() {
        return 'crate_owner_invite';
    },
});
