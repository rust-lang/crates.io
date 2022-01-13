import { belongsTo, Model } from 'miragejs';

export default Model.extend({
  crate: belongsTo(),
  invitee: belongsTo('user'),
  inviter: belongsTo('user'),
});
