import { Model, hasMany } from 'ember-cli-mirage';

export default Model.extend({
  categories: hasMany(),
  keywords: hasMany(),
  teamOwners: hasMany('team'),
  versions: hasMany(),
  userOwners: hasMany('user'),
});
