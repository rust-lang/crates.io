import Ember from 'ember';

function fromNow(value) {
    return moment(value).fromNow();
}

export {
    fromNow
};

export default Ember.Handlebars.makeBoundHelper(fromNow);
