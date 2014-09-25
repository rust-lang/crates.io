import DS from 'ember-data';

export default DS.Model.extend({
  num: DS.attr('string'),
  url: DS.attr('string'),
  pkg: DS.belongsTo('package'),
  updated_at: DS.attr('date'),
  created_at: DS.attr('date'),
});
