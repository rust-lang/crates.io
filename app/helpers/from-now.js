import Ember from 'ember';
import moment from 'moment';

function fromNow(value) {
    return moment(value).fromNow();
}

export {
    fromNow
};

export default Ember.Handlebars.makeBoundHelper(fromNow);
