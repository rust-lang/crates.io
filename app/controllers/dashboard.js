import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { inject as service } from '@ember/service';

const TO_SHOW = 5;

export default Controller.extend({

    ajax: service(),

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

    visibleCrates: computed('myCrates.[]', function() {
        return this.get('myCrates').slice(0, TO_SHOW);
    }),

    visibleFollowing: computed('myFollowing.[]', function() {
        return this.get('myFollowing').slice(0, TO_SHOW);
    }),

    visibleStats: computed('myStats', function() {
        return this.get('myStats');
    }),

    hasMoreCrates: computed('myCrates.[]', function() {
        return this.get('myCrates.length') > TO_SHOW;
    }),

    hasMoreFollowing: computed('myFollowing.[]', function() {
        return this.get('myFollowing.length') > TO_SHOW;
    }),

    actions: {
        loadMore() {
            this.set('loadingMore', true);
            let page = (this.get('myFeed').length / 10) + 1;

            this.get('ajax').request(`/api/v1/me/updates?page=${page}`).then((data) => {
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
