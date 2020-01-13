import Model, { attr, hasMany } from '@ember-data/model';

export default Model.extend({
  category: attr('string'),
  slug: attr('string'),
  description: attr('string'),
  created_at: attr('date'),
  crates_cnt: attr('number'),

  subcategories: attr(),
  parent_categories: attr(),

  crates: hasMany('crate', { async: true }),
});
