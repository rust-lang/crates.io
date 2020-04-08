import { Model, hasMany } from 'ember-cli-mirage';

export default Model.extend({
  followedCrates: hasMany('crate'),
});
