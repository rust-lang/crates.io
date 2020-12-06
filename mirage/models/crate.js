import { hasMany, Model } from 'ember-cli-mirage';

export default Model.extend({
  categories: hasMany(),
  keywords: hasMany(),
  versions: hasMany(),
});
