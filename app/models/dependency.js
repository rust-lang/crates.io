import DS from 'ember-data';
import Ember from 'ember';

Ember.Inflector.inflector.irregular('dependency', 'dependencies');

export default DS.Model.extend({
    version: DS.belongsTo('version'),
    crate_id: DS.attr('string'),
    req: DS.attr('string'),
    optional: DS.attr('boolean'),
    default_features: DS.attr('boolean'),
    features: DS.attr('string'),

    featureList: function() {
        return this.get('features').split(',');
    }.property('features'),
});
