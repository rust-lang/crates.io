import Ember from 'ember';

function dateLong(value) {
    return moment(value).format('LL');
}

export {
    dateLong
};

export default Ember.Handlebars.makeBoundHelper(dateLong);
