import Ember from 'ember';
import moment from 'moment';

function dateSmall(value) {
    return moment(value).format('ll');
}

export {
    dateSmall
};

export default Ember.Handlebars.makeBoundHelper(dateSmall);
