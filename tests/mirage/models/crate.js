import { hasMany, Model } from 'miragejs';

export default Model.extend({
  categories: hasMany(),
  keywords: hasMany(),
  versions: hasMany(),
});
