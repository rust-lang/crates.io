import Ember from 'ember';

export default Ember.Controller.extend({
    flashError: null,
    nextFlashError: null,
    showUserOptions: false,

    stepFlash: function() {
        this.set('flashError', this.get('nextFlashError'));
        this.set('nextFlashError', null);
    },

    aboutToTransition: function() {
        Ember.$(document).trigger('mousedown');
    },

    resetDropdownOption: function(controller, option) {
        controller.set(option, !controller.get(option));
        if (controller.get(option)) {
            Ember.$(document).on('mousedown.useroptions', function(e) {
                if (Ember.$(e.target).prop('tagName') === 'INPUT') {
                    return;
                }
                Ember.run.later(function() {
                    controller.set(option, false);
                }, 150);
                Ember.$(document).off('mousedown.useroptions');
            });
        }
    },

    currentPathChanged: function () {
        window.scrollTo(0, 0);
    }.observes('currentPath'),

    actions: {
        search: function(query) {
            this.transitionToRoute('search', {
              queryParams: {q: query, page: 1}
            });
        },

        toggleUserOptions: function() {
            this.resetDropdownOption(this, 'showUserOptions');
        },
    },
});

