import Ember from "ember";

var escape = Ember.Handlebars.Utils.escapeExpression;

function formatEmail(email) {
  var formatted = email.match(/^(.*?)\s*(?:<(.*)>)?$/);
  var email = "";

  email += escape(formatted[1]);

  if (formatted[2]) {
    email = "<a href='mailto:" + escape(formatted[2]) + "'>" + email + "</a>";
  }

  console.log(email);
  return email.htmlSafe();
}

export default Ember.Handlebars.makeBoundHelper(formatEmail);
