import Ember from 'ember';

function dateSmall(value) {
    return moment(value).format('ll');
}

export {
    dateSmall
};

export default Ember.Handlebars.makeBoundHelper(dateSmall);
