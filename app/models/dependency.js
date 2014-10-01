import DS from 'ember-data';

export default DS.Model.extend({
    version: DS.belongsTo('version'),
    crate: DS.belongsTo('crate'),
    req: DS.attr('string'),
    optional: DS.attr('boolean'),
    default_features: DS.attr('boolean'),
    features: DS.attr('string'),

    featureList: function() {
        return this.get('features').split(',');
    }.property('features'),
});
