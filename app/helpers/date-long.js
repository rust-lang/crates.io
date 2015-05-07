import Ember from 'ember';
import moment from 'moment';

function dateLong(value) {
    return moment(value).format('LL');
}

export {
    dateLong
};

export default Ember.Handlebars.makeBoundHelper(dateLong);
