import DS from 'ember-data';

export default DS.RESTSerializer.extend({
    extractId(modelClass, resourceHash) {
        return `${resourceHash.date}-${resourceHash.version}`;
    },
});
