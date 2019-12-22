import Model, { attr } from '@ember-data/model';

export default Model.extend({
  name: attr('string'),
  token: attr('string'),
  created_at: attr('date'),
  last_used_at: attr('date'),
});
