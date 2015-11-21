import Ember from 'ember';

const { observer } = Ember;

export default Ember.Controller.extend({
    searchController: Ember.inject.controller('search'),

    flashError: null,
    nextFlashError: null,
    search: Ember.computed.oneWay('searchController.q'),

    stepFlash() {
        this.set('flashError', this.get('nextFlashError'));
        this.set('nextFlashError', null);
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
    },
});

