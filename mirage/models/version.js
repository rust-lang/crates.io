import { belongsTo, Model } from 'miragejs';

export default Model.extend({
  crate: belongsTo(),
  publishedBy: belongsTo('user'),
});
