import { A } from '@ember/array';
import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

import { task } from 'ember-concurrency';

import ajax from '../utils/ajax';

const TO_SHOW = 5;

export default class DashboardController extends Controller {
  hasMore = false;
  myFeed = A();

  @alias('model.myCrates') myCrates;
  @alias('model.myFollowing') myFollowing;
  @alias('model.myStats') myStats;

  @computed('myCrates.[]')
  get visibleCrates() {
    return this.myCrates.slice(0, TO_SHOW);
  }

  @computed('myFollowing.[]')
  get visibleFollowing() {
    return this.myFollowing.slice(0, TO_SHOW);
  }

  @computed('myCrates.[]')
  get hasMoreCrates() {
    return this.get('myCrates.length') > TO_SHOW;
  }

  @computed('myFollowing.[]')
  get hasMoreFollowing() {
    return this.get('myFollowing.length') > TO_SHOW;
  }

  @task(function* () {
    let page = this.myFeed.length / 10 + 1;

    let data = yield ajax(`/api/v1/me/updates?page=${page}`);
    let versions = data.versions.map(version => this.store.push(this.store.normalize('version', version)));

    this.myFeed.pushObjects(versions);
    this.set('hasMore', data.meta.more);
  })
  loadMoreTask;
}
