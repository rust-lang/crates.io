import Model, { attr } from '@ember-data/model';

export default class ApiToken extends Model {
  @attr name;
  @attr token;
  @attr('date') created_at;
  @attr('date') last_used_at;
}
