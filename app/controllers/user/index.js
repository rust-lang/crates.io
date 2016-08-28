import Ember from 'ember';

const TO_SHOW = 5;
const { computed } = Ember;

export default Ember.Controller.extend({
    init() {
        this._super(...arguments);

        this.fetchingFeed = true;
        this.loadingMore = false;
        this.hasMore = false;
        this.crates = [];
    },

    visibleCrates: computed('crates', function() {
        return this.get('crates').slice(0, TO_SHOW);
    }),

    hasMoreCrates: computed('crates', function() {
        return this.get('crates.length') > TO_SHOW;
    })
});
