import Ember from 'ember';
import moment from 'moment';

export function dateSmall(value) {
    return moment(value).format('ll');
}

export default Ember.Helper.helper(params => dateSmall(params[0]));
