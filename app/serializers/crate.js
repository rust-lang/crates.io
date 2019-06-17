import DS from 'ember-data';

export default DS.RESTSerializer.extend({
    isNewSerializerAPI: true,

    extractRelationships(modelClass, resourceHash) {
        if (resourceHash.versions == null) {
            delete resourceHash.versions;
        }

        return this._super(...arguments);
    },
});
