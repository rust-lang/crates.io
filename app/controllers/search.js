import Controller from '@ember/controller';
import { action } from '@ember/object';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { restartableTask } from 'ember-concurrency';
import { bool, reads } from 'macro-decorators';

import { pagination } from '../utils/pagination';
import { CATEGORY_PREFIX, processSearchQuery } from '../utils/search';

export default class SearchController extends Controller {
  @service store;

  queryParams = ['all_keywords', 'page', 'per_page', 'q', 'sort'];
  @tracked all_keywords;
  @tracked q = '';
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

  @pagination() pagination;

  get pageTitle() {
    return 'Search Results' + (this.q ? ` for '${this.q}'` : '');
  }

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

  get hasMultiCategoryFilter() {
    let tokens = this.q.trim().split(/\s+/);
    return tokens.filter(token => token.startsWith(CATEGORY_PREFIX)).length > 1;
  }

  @action fetchData() {
    this.dataTask.perform().catch(() => {
      // we ignore errors here because they are handled in the template already
    });
  }

  dataTask = restartableTask(async () => {
    let { all_keywords, page, per_page, q, sort } = this;

    let query = q.trim();

    let searchOptions = all_keywords
      ? { page, per_page, sort, q: query, all_keywords }
      : { page, per_page, sort, ...processSearchQuery(query) };

    return await this.store.query('crate', searchOptions);
  });

  get totalItems() {
    return this.model.meta.total ?? 0;
  }
}
