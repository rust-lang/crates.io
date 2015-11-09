import Ember from 'ember';
import moment from 'moment';

export function dateLong(value) {
    return moment(value).format('LL');
}

export default Ember.Helper.helper(params => dateLong(params[0]));
