import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import { bool, reads } from 'macro-decorators';

import { pagination } from '../utils/pagination';

export default class SearchController extends Controller {
  @service store;

  queryParams = ['all_keywords', 'page', 'per_page', 'q', 'sort'];
  @tracked all_keywords;
  @tracked q = null;
  @tracked page = '1';
  @tracked per_page = 10;
  @tracked sort;

  @reads('dataTask.lastSuccessful.value') model;

  get hasData() {
    return this.dataTask.lastSuccessful || !this.dataTask.isRunning;
  }

  get firstResultPending() {
    return !this.dataTask.lastComplete && this.dataTask.isRunning;
  }

  @reads('model.meta.total') totalItems;

  @pagination() pagination;

  get currentSortBy() {
    if (this.sort === 'downloads') {
      return 'All-Time Downloads';
    } else if (this.sort === 'recent-downloads') {
      return 'Recent Downloads';
    } else if (this.sort === 'recent-updates') {
      return 'Recent Updates';
    } else if (this.sort === 'new') {
      return 'Newly Added';
    } else {
      return 'Relevance';
    }
  }

  @bool('totalItems') hasItems;

  @action fetchData() {
    this.dataTask.perform().catch(() => {
      // we ignore errors here because they are handled in the template already
    });
  }

  @(task(function* () {
    let { all_keywords, page, per_page, q, sort } = this;

    if (q !== null) {
      q = q.trim();
    }

    return yield this.store.query('crate', { all_keywords, page, per_page, q, sort });
  }).restartable())
  dataTask;

  get exactMatch() {
    return this.model.find(it => it.exact_match);
  }

  get otherCrates() {
    return this.model.filter(it => !it.exact_match);
  }
}
