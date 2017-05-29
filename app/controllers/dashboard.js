import Ember from 'ember';
import ajax from 'ic-ajax';

const TO_SHOW = 5;
const { computed } = Ember;

export default Ember.Controller.extend({
    init() {
        this._super(...arguments);

        this.fetchingFeed = true;
        this.loadingMore = false;
        this.hasMore = false;
        this.myCrates = [];
        this.myFollowing = [];
        this.myFeed = [];
        this.myStats = 0;
    },

    visibleCrates: computed('myCreates', function() {
        return this.get('myCrates').slice(0, TO_SHOW);
    }),

    visibleFollowing: computed('myFollowing', function() {
        return this.get('myFollowing').slice(0, TO_SHOW);
    }),

    visibleStats: computed('myStats', function() {
        return this.get('myStats');
    }),

    hasMoreCrates: computed('myCreates', function() {
        return this.get('myCrates.length') > TO_SHOW;
    }),

    hasMoreFollowing: computed('myFollowing', function() {
        return this.get('myFollowing.length') > TO_SHOW;
    }),

    actions: {
        loadMore() {
            this.set('loadingMore', true);
            var page = (this.get('myFeed').length / 10) + 1;

            ajax(`/me/updates?page=${page}`).then((data) => {
                var versions = data.versions.map(version =>
                    this.store.push(this.store.normalize('version', version)));

                this.get('myFeed').pushObjects(versions);
                this.set('hasMore', data.meta.more);
            }).finally(() => {
                this.set('loadingMore', false);
            });
        }
    }
});
