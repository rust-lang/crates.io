import Ember from 'ember';
import moment from 'moment';

export default Ember.Helper.helper(function(params) {
    let value = params[0];
    return moment(value).fromNow();
});
