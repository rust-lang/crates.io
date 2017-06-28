import Ember from 'ember';

export default Ember.Service.extend({
    flashError: null,
    nextFlashError: null,

    show(message) {
        this.set('flashError', message);
    },

    queue(message) {
        this.set('nextFlashError', message);
    },

    stepFlash() {
        this.set('flashError', this.get('nextFlashError'));
        this.set('nextFlashError', null);
    }
});
