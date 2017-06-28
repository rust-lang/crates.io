import Ember from 'ember';

export default Ember.Component.extend({
    tagName: '',

    didUpdate() {
        window.scrollTo(0, 0);
    }
});
