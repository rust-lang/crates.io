import Model, { belongsTo, attr } from '@ember-data/model';

import Inflector from 'ember-inflector';

Inflector.inflector.irregular('dependency', 'dependencies');

export default class Dependency extends Model {
  @attr('string') crate_id;
  @attr('string') req;
  @attr('boolean') optional;
  @attr('boolean') default_features;
  @attr({ defaultValue: () => [] }) features;
  @attr('string') kind;
  @attr('number') downloads;

  @belongsTo('version', { async: false }) version;
}
