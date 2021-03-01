import Controller from '@ember/controller';
import { action, computed } from '@ember/object';
import { bool, readOnly } from '@ember/object/computed';

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

  @computed('dataTask.{lastComplete,isRunning}')
  get firstResultPending() {
    return !this.dataTask.lastComplete && this.dataTask.isRunning;
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
