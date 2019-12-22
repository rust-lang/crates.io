import Model, { attr } from '@ember-data/model';

export default Model.extend({
  invited_by_username: attr('string'),
  crate_name: attr('string'),
  crate_id: attr('number'),
  created_at: attr('date'),
  accepted: attr('boolean', { defaultValue: false }),
});
