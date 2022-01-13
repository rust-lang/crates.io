import { belongsTo, Model } from 'miragejs';

export default Model.extend({
  crate: belongsTo(),
  team: belongsTo(),
  user: belongsTo(),
});
