import Model, { attr, hasMany } from '@ember-data/model';

export default class Category extends Model {
  @attr category;
  @attr slug;
  @attr description;
  @attr('date') created_at;
  @attr crates_cnt;

  @attr subcategories;
  @attr parent_categories;

  @hasMany('crate', { async: true }) crates;
}
