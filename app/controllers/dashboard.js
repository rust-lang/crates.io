import Controller from '@ember/controller';
import { A } from '@ember/array';
import { computed } from '@ember/object';
import ajax from 'ember-fetch/ajax';

const TO_SHOW = 5;

export default Controller.extend({
  init() {
    this._super(...arguments);

    this.loadingMore = false;
    this.hasMore = false;
    this.myCrates = A();
    this.myFollowing = A();
    this.myFeed = A();
    this.myStats = 0;
  },

  visibleCrates: computed('myCrates.[]', function() {
    return this.myCrates.slice(0, TO_SHOW);
  }),

  visibleFollowing: computed('myFollowing.[]', function() {
    return this.myFollowing.slice(0, TO_SHOW);
  }),

  visibleStats: computed('myStats', function() {
    return this.myStats;
  }),

  hasMoreCrates: computed('myCrates.[]', function() {
    return this.get('myCrates.length') > TO_SHOW;
  }),

  hasMoreFollowing: computed('myFollowing.[]', function() {
    return this.get('myFollowing.length') > TO_SHOW;
  }),

  actions: {
    async loadMore() {
      this.set('loadingMore', true);
      let page = this.myFeed.length / 10 + 1;

      try {
        let data = await ajax(`/api/v1/me/updates?page=${page}`);
        let versions = data.versions.map(version => this.store.push(this.store.normalize('version', version)));

        this.myFeed.pushObjects(versions);
        this.set('hasMore', data.meta.more);
      } finally {
        this.set('loadingMore', false);
      }
    },
  },
});
