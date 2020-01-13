import Model, { attr } from '@ember-data/model';

export default Model.extend({
  name: attr('string'),
  email_notifications: attr('boolean'),
});
