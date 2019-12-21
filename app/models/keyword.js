import Model, { attr, hasMany } from '@ember-data/model';

export default Model.extend({
  keyword: attr('string'),
  created_at: attr('date'),
  crates_cnt: attr('number'),

  crates: hasMany('crate', { async: true }),
});
