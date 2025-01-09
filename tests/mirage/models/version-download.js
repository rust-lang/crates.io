import { belongsTo, Model } from 'miragejs';

export default Model.extend({
  version: belongsTo(),
});
