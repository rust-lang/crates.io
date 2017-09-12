import DS from 'ember-data';

export default DS.RESTSerializer.extend({
    modelNameFromPayloadKey() {
        return 'crate-owner-invite';
    }
});
