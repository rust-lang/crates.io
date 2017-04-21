import Ember from 'ember';

export function buildInfoPassFail(status) {
    if (status === true) {
        return '✅ Pass';
    } else if (status === false) {
        return '❌ Fail';
    } else {
        return '';
    }
}

export default Ember.Helper.helper(params => buildInfoPassFail(params[0]));
