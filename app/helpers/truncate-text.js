import Ember from 'ember';

export default Ember.Helper.helper(function(params) {
    let [value] = params;
    if (!value) {
        return value;
    }
    if (value.length > 200) {
        return `${value.slice(0, 200)} ...`;
    }
    return value;
});
