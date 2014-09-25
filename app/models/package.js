import DS from 'ember-data';

export default DS.Model.extend({
  name: DS.attr('string'),
  versions: DS.hasMany('versions', {async:true}),
  created_at: DS.attr('date'),
  updated_at: DS.attr('date'),
});
