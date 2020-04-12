import Model, { attr } from '@ember-data/model';

export default class CrateOwnerInvite extends Model {
  @attr('string') invited_by_username;
  @attr('string') crate_name;
  @attr('number') crate_id;
  @attr('date') created_at;
  @attr('boolean', { defaultValue: false }) accepted;
}
