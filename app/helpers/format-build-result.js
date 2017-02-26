import Ember from 'ember';

export function formatBuildResult(result) {
    if (result === true) {
        return 'Pass';
    } else if (result === false) {
        return 'Fail';
    } else {
        return null;
    }
}

export default Ember.Helper.helper(params => formatBuildResult(params[0]));
