import Ember from 'ember';

var escape = Ember.Handlebars.Utils.escapeExpression;

export function formatEmail(email) {
    var formatted = email.match(/^(.*?)\s*(?:<(.*)>)?$/);
    var ret = '';

    ret += escape(formatted[1]);

    if (formatted[2]) {
        ret = `<a href='mailto:${escape(formatted[2])}'>${ret}</a>`;
    }

    return ret.htmlSafe();
}

export default Ember.Helper.helper(params => formatEmail(params[0]));
