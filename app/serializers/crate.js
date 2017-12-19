import DS from 'ember-data';

export default DS.RESTSerializer.extend({
    isNewSerializerAPI: true,
    attrs: {
        originalDocumentation: 'documentation'
    },

    extractRelationships(modelClass, resourceHash) {
        if (resourceHash.versions == null) {
            delete resourceHash.versions;
        }

        return this._super(...arguments);
    }
});
