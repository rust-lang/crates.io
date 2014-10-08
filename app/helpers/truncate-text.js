import Ember from 'ember';

function truncateText(value) {
    if (!value) { return value; }
    if (value.length > 200) {
        return value.slice(0, 200) + ' ...';
    }
    return value;
}

export {
    truncateText
};

export default Ember.Handlebars.makeBoundHelper(truncateText);
