import Model, { attr } from '@ember-data/model';

export default Model.extend({
  slug: attr('string'),
  description: attr('string'),
});
