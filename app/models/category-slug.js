import Model, { attr } from '@ember-data/model';

export default class CategorySlug extends Model {
  @attr('string') slug;
  @attr('string') description;
}
