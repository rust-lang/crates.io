import Model, { attr } from '@ember-data/model';

export default class ApiToken extends Model {
  @attr('string') name;
  @attr('string') token;
  @attr('date') created_at;
  @attr('date') last_used_at;
}
