import Ember from 'ember';

const { observer } = Ember;

export default Ember.Controller.extend({
    searchController: Ember.inject.controller('search'),

    flashError: null,
    nextFlashError: null,
    showUserOptions: false,
    search: Ember.computed.oneWay('searchController.q'),

    stepFlash() {
        this.set('flashError', this.get('nextFlashError'));
        this.set('nextFlashError', null);
    },

    aboutToTransition() {
        Ember.$(document).trigger('mousedown');
    },

    // don't use this from other controllers..
    resetDropdownOption(controller, option) {
        controller.set(option, !controller.get(option));
        if (controller.get(option)) {
            Ember.$(document).on('mousedown.useroptions', (e) => {
                if (Ember.$(e.target).prop('tagName') === 'INPUT') {
                    return;
                }
                Ember.run.later(() => {
                    controller.set(option, false);
                }, 150);
                Ember.$(document).off('mousedown.useroptions');
            });
        }
    },

    _scrollToTop() {
        window.scrollTo(0, 0);
    },

    // TODO: remove observer & DOM mutation in controller..
    currentPathChanged: observer('currentPath', function () {
      Ember.run.scheduleOnce('afterRender', this, this._scrollToTop);
    }),

    actions: {
        search(q) {
            this.transitionToRoute('search', {
              queryParams: {
                q,
                page: 1
              }
            });
        },

        toggleUserOptions() {
            this.resetDropdownOption(this, 'showUserOptions');
        },
    },
});

