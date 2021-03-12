import Model, { attr, belongsTo } from '@ember-data/model';

import Inflector from 'ember-inflector';

Inflector.inflector.irregular('dependency', 'dependencies');

export default class Dependency extends Model {
  @attr req;
  @attr optional;
  @attr default_features;
  @attr({ defaultValue: () => [] }) features;
  @attr kind;
  @attr downloads;

  @belongsTo('version', { async: false }) version;
  @belongsTo({ async: true }) crate;

  get crate_id() {
    return this.belongsTo('crate').id();
  }
}
