import Model, { belongsTo, attr } from '@ember-data/model';
import Inflector from 'ember-inflector';

Inflector.inflector.irregular('dependency', 'dependencies');

export default Model.extend({
  version: belongsTo('version', {
    async: false,
  }),
  crate_id: attr('string'),
  req: attr('string'),
  optional: attr('boolean'),
  default_features: attr('boolean'),
  features: attr({ defaultValue: () => [] }),
  kind: attr('string'),
  downloads: attr('number'),
});
