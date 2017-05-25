import DS from 'ember-data';

export default DS.RESTSerializer.extend({
    normalize(modelClass, resourceHash) {
        modelClass.eachRelationship(key => resourceHash[key] !== null || delete resourceHash[key]);
        return this._super(...arguments);
    }
});
