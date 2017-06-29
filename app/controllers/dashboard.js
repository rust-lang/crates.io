import Ember from 'ember';

const TO_SHOW = 5;
const { computed, inject: { service } } = Ember;

export default Ember.Controller.extend({

    ajax: service(),

    init() {
        this._super(...arguments);

        this.fetchingFeed = true;
        this.loadingMore = false;
        this.hasMore = false;
        this.myCrates = [];
        this.myFollowing = [];
        this.myFeed = [];
    },

    visibleCrates: computed('myCreates', function() {
        return this.get('myCrates').slice(0, TO_SHOW);
    }),

    visibleFollowing: computed('myFollowing', function() {
        return this.get('myFollowing').slice(0, TO_SHOW);
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
            let page = (this.get('myFeed').length / 10) + 1;

            this.get('ajax').request(`/me/updates?page=${page}`).then((data) => {
                let versions = data.versions.map(version =>
                    this.store.push(this.store.normalize('version', version)));

                this.get('myFeed').pushObjects(versions);
                this.set('hasMore', data.meta.more);
            }).finally(() => {
                this.set('loadingMore', false);
            });
        }
    }
});
