import Ember from "ember";

export default Ember.Helper.helper(function(params) {
  let req = params[0];
  return req === "*" ? "" : req;
});
