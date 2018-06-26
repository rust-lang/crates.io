import DS from 'ember-data';

export default DS.Model.extend({
    version: DS.belongsTo('version', {
        async: false,
    }),
    downloads: DS.attr('number'),
    date: DS.attr('date'),
});
