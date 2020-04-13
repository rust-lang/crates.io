import { A } from '@ember/array';
import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

import { task } from 'ember-concurrency';

import ajax from '../utils/ajax';

const TO_SHOW = 5;

export default Controller.extend({
  init() {
    this._super(...arguments);

    this.loadingMore = false;
    this.hasMore = false;
    this.myFeed = A();
  },

  myCrates: alias('model.myCrates'),
  myFollowing: alias('model.myFollowing'),
  myStats: alias('model.myStats'),

  visibleCrates: computed('myCrates.[]', function () {
    return this.myCrates.slice(0, TO_SHOW);
  }),

  visibleFollowing: computed('myFollowing.[]', function () {
    return this.myFollowing.slice(0, TO_SHOW);
  }),

  hasMoreCrates: computed('myCrates.[]', function () {
    return this.get('myCrates.length') > TO_SHOW;
  }),

  hasMoreFollowing: computed('myFollowing.[]', function () {
    return this.get('myFollowing.length') > TO_SHOW;
  }),

  loadMoreTask: task(function* () {
    this.set('loadingMore', true);
    let page = this.myFeed.length / 10 + 1;

    try {
      let data = yield ajax(`/api/v1/me/updates?page=${page}`);
      let versions = data.versions.map(version => this.store.push(this.store.normalize('version', version)));

      this.myFeed.pushObjects(versions);
      this.set('hasMore', data.meta.more);
    } finally {
      this.set('loadingMore', false);
    }
  }),
});
