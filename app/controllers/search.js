import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias, bool, readOnly } from '@ember/object/computed';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';

import PaginationMixin from '../mixins/pagination';

export default Controller.extend(PaginationMixin, {
  search: service(),
  queryParams: ['all_keywords', 'page', 'per_page', 'q', 'sort'],
  q: alias('search.q'),
  page: '1',
  per_page: 10,

  model: readOnly('dataTask.lastSuccessful.value'),

  hasData: computed('dataTask.{lastSuccessful,isRunning}', function() {
    return this.get('dataTask.lastSuccessful') || !this.get('dataTask.isRunning');
  }),

  firstResultPending: computed('dataTask.{lastSuccessful,isRunning}', function() {
    return !this.get('dataTask.lastSuccessful') && this.get('dataTask.isRunning');
  }),

  totalItems: readOnly('model.meta.total'),

  currentSortBy: computed('sort', function() {
    if (this.sort === 'downloads') {
      return 'All-Time Downloads';
    } else if (this.sort === 'recent-downloads') {
      return 'Recent Downloads';
    } else if (this.get('sort') === 'recent-updates') {
      return 'Recent Updates';
    } else {
      return 'Relevance';
    }
  }),

  hasItems: bool('totalItems'),

  dataTask: task(function*(params) {
    if (params.q !== null) {
      params.q = params.q.trim();
    }

    return yield this.store.query('crate', params);
  }).drop(),
});
