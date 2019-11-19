import DS from 'ember-data';

export default DS.Model.extend({
  name: DS.attr('string'),
  email_notifications: DS.attr('boolean'),
});
