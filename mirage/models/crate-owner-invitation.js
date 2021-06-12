import { belongsTo, Model } from 'ember-cli-mirage';

export default Model.extend({
  crate: belongsTo(),
  invitee: belongsTo('user'),
  inviter: belongsTo('user'),
});
