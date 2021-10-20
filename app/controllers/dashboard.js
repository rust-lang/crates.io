import { A } from '@ember/array';
import Controller from '@ember/controller';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';
import { alias } from 'macro-decorators';

import ajax from '../utils/ajax';

const TO_SHOW = 5;

export default class DashboardController extends Controller {
  @service store;

  hasMore = false;
  myFeed = A();

  @alias('model.myCrates') myCrates;
  @alias('model.myFollowing') myFollowing;
  @alias('model.myStats') myStats;

  get visibleCrates() {
    return this.myCrates.slice(0, TO_SHOW);
  }

  get visibleFollowing() {
    return this.myFollowing.slice(0, TO_SHOW);
  }

  get hasMoreCrates() {
    return this.myCrates.length > TO_SHOW;
  }

  get hasMoreFollowing() {
    return this.myFollowing.length > TO_SHOW;
  }

  @task *loadMoreTask() {
    let page = this.myFeed.length / 10 + 1;

    let data = yield ajax(`/api/v1/me/updates?page=${page}`);
    let versions = data.versions.map(version => this.store.push(this.store.normalize('version', version)));

    this.myFeed.pushObjects(versions);
    this.set('hasMore', data.meta.more);
  }
}
