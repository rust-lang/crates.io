import Model, { attr, belongsTo } from '@ember-data/model';
import { irregular } from '@ember-data/request-utils/string';

irregular('dependency', 'dependencies');

export default class Dependency extends Model {
  @attr crate_id;
  @attr req;
  @attr optional;
  @attr default_features;
  @attr({ defaultValue: () => [] }) features;
  @attr kind;
  @attr downloads;

  @belongsTo('version', { async: false, inverse: 'dependencies' }) version;
}
