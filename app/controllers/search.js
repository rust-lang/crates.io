import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly, bool } from '@ember/object/computed';

import { task } from 'ember-concurrency';

import { pagination } from '../utils/pagination';

export default class SearchController extends Controller {
  queryParams = ['all_keywords', 'page', 'per_page', 'q', 'sort'];
  q = null;
  page = '1';
  per_page = 10;

  @readOnly('dataTask.lastSuccessful.value') model;

  @computed('dataTask.{lastSuccessful,isRunning}')
  get hasData() {
    return this.dataTask.lastSuccessful || !this.dataTask.isRunning;
  }

  @computed('dataTask.{lastSuccessful,isRunning}')
  get firstResultPending() {
    return !this.dataTask.lastSuccessful && this.dataTask.isRunning;
  }

  @readOnly('model.meta.total') totalItems;

  @pagination() pagination;

  @computed('sort')
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

  @(task(function* () {
    let { all_keywords, page, per_page, q, sort } = this;

    if (q !== null) {
      q = q.trim();
    }

    return yield this.store.query('crate', { all_keywords, page, per_page, q, sort });
  }).drop())
  dataTask;
}
