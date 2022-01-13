import { belongsTo, Model } from 'miragejs';

/**
 * This is a mirage-only model, that is used to keep track of the current
 * session and the associated `user` model, because in route handlers we don't
 * have access to the cookie data that the actual API is using for these things.
 *
 * This mock implementation means that there can only ever exist one
 * session at a time.
 */
export default Model.extend({
  user: belongsTo(),
});
