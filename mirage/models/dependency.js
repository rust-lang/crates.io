import { Model, belongsTo } from 'ember-cli-mirage';

export default Model.extend({
  crate: belongsTo(),
  version: belongsTo(),
});
