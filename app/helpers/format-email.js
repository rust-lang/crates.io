import Ember from 'ember';

const escape = Ember.Handlebars.Utils.escapeExpression;

export function formatEmail(email) {
    let formatted = email.match(/^(.*?)\s*(?:<(.*)>)?$/);
    let ret = '';

    ret += escape(formatted[1]);

    if (formatted[2]) {
        ret = `<a href='mailto:${escape(formatted[2])}'>${ret}</a>`;
    }

    return ret.htmlSafe();
}

export default Ember.Helper.helper(params => formatEmail(params[0]));
