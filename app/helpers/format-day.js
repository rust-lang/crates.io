import Ember from 'ember';
import moment from 'moment';

export function formatDay(date) {
    return date ? moment(date).utc().format('YYYY-MM-DD') : null;
}

export default Ember.Helper.helper(params => formatDay(params[0]));
