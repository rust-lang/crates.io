import DS from 'ember-data';
import Ember from 'ember';

Ember.Inflector.inflector.irregular('dependency', 'dependencies');

const { computed } = Ember;

export default DS.Model.extend({
    version: DS.belongsTo('version', {
        async: false
    }),
    crate_id: DS.attr('string'),
    req: DS.attr('string'),
    optional: DS.attr('boolean'),
    default_features: DS.attr('boolean'),
    features: DS.attr('string'),
    kind: DS.attr('string'),

    featureList: computed('features', function() {
        return this.get('features').split(',');
    })
});
