import Model, { attr } from '@ember-data/model';

export default class OwnedCrate extends Model {
  @attr name;
  @attr email_notifications;
}
