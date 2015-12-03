import Ember from 'ember';

export default Ember.Helper.helper(function(params) {
    let [req] = params;
    return req === '*' ? '' : req;
});
