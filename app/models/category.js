import Model, { attr, hasMany } from '@ember-data/model';

export default class Category extends Model {
  @attr('string') category;
  @attr('string') slug;
  @attr('string') description;
  @attr('date') created_at;
  @attr('number') crates_cnt;

  @attr() subcategories;
  @attr() parent_categories;

  @hasMany('crate', { async: true }) crates;
}
