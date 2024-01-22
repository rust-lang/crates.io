import Model, { attr, belongsTo } from '@ember-data/model';

export default class CrateOwnerInvite extends Model {
  @attr crate_name;
  @attr crate_id;
  @attr('date') created_at;
  @attr accepted;
  @belongsTo('user', { async: false, inverse: null }) invitee;
  @belongsTo('user', { async: false, inverse: null }) inviter;
}
