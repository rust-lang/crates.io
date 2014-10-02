import Ember from 'ember';

export default Ember.Controller.extend({
    flashError: null,
    nextFlashError: null,
    showUserOptions: false,

    stepFlash: function() {
        this.set('flashError', this.get('nextFlashError'));
        this.set('nextFlashError', null);
    },

    resetDropdownOption: function(controller, option) {
        controller.set(option, !controller.get(option));
        if (controller.get(option)) {
            Ember.$(document).on('mousedown.useroptions', function() {
                Ember.run.later(function() {
                    controller.set(option, false);
                }, 100);
                Ember.$(document).off('mousedown.useroptions');
            });
        }
    },

    actions: {
        search: function(query) {
            this.transitionToRoute('search', {queryParams: {q: query}});
        },

        toggleUserOptions: function() {
            this.resetDropdownOption(this, 'showUserOptions');
        },
    },
});

