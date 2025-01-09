import { hasMany, Model } from 'miragejs';

export default Model.extend({
  followedCrates: hasMany('crate'),
});
