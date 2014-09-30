import Ember from 'ember';

export default Ember.Controller.extend({
    flashError: null,
    nextFlashError: null,

    stepFlash: function() {
        this.set('flashError', this.get('nextFlashError'));
        this.set('nextFlashError', null);
    },

    actions: {
        search: function(query) {
            this.transitionToRoute('search', {queryParams: {q: query}});
        },
    },
});

