import Ember from "ember";

function formatReq(req) {
  return req === "*" ? "" : req;
}

export default Ember.Handlebars.makeBoundHelper(formatReq);
