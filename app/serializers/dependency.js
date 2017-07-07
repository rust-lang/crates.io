import DS from 'ember-data';

export default DS.RESTSerializer.extend({
    attrs: {
        version: 'version_id',
    },
});
