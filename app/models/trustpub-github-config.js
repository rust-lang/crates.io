import Model, { attr, belongsTo } from '@ember-data/model';

export default class TrustpubGitHubConfig extends Model {
  @belongsTo('crate', { async: true, inverse: null }) crate;
  @attr repository_owner;
  @attr repository_owner_id;
  @attr repository_name;
  @attr workflow_filename;
  @attr environment;
  @attr('date') created_at;
}
