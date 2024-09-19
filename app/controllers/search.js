import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { all, restartableTask } from 'ember-concurrency';
import { bool, reads } from 'macro-decorators';

import { AjaxError } from '../utils/ajax';
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

  @reads('model.meta.total') totalItems;

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

    const crates = await this.store.query('crate', searchOptions);

    // Prime the docs for the most recent versions of each crate.
    const docTasks = [];
    for (const crate of crates) {
      docTasks.push(crate.loadDocsStatusTask.perform());
    }
    try {
      await all(docTasks);
    } catch (e) {
      // report unexpected errors to Sentry and ignore `ajax()` errors
      if (!didCancel(error) && !(error instanceof AjaxError)) {
        this.sentry.captureException(error);
      }
    }

    return crates;
  });
}
