import DS from 'ember-data';
import Inflector from 'ember-inflector';

Inflector.inflector.irregular('dependency', 'dependencies');

export default DS.Model.extend({
    version: DS.belongsTo('version', {
        async: false,
    }),
    crate_id: DS.attr('string'),
    req: DS.attr('string'),
    optional: DS.attr('boolean'),
    default_features: DS.attr('boolean'),
    features: DS.attr({ defaultValue: () => [] }),
    kind: DS.attr('string'),
    downloads: DS.attr('number'),
});
