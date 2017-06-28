import Ember from 'ember';

export default Ember.Component.extend({
    didUpdate() {
        window.scrollTo(0, 0);
    }
});
