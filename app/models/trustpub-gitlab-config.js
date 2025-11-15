import Model, { attr, belongsTo } from '@ember-data/model';

export default class TrustpubGitLabConfig extends Model {
  @belongsTo('crate', { async: true, inverse: null }) crate;
  @attr namespace;
  @attr namespace_id;
  @attr project;
  @attr workflow_filepath;
  @attr environment;
  @attr('date') created_at;
}
