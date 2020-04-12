import Model, { attr } from '@ember-data/model';

export default class OwnedCrate extends Model {
  @attr('string') name;
  @attr('boolean') email_notifications;
}
