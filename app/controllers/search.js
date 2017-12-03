import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias, bool, readOnly } from '@ember/object/computed';
import { inject as service } from '@ember/service';

import { task, timeout } from 'ember-concurrency';

import PaginationMixin from '../mixins/pagination';

const DEBOUNCE_MS = 250;

export default Controller.extend(PaginationMixin, {
    search: service(),
    queryParams: ['q', 'page', 'per_page', 'sort'],
    q: alias('search.q'),
    page: '1',
    per_page: 10,
    sort: null,

    model: readOnly('dataTask.lastSuccessful.value'),

    hasData: computed('dataTask.lastSuccessful', 'dataTask.isRunning', function() {
        return this.get('dataTask.lastSuccessful') || !this.get('dataTask.isRunning');
    }),

    firstResultPending: computed('dataTask.lastSuccessful', 'dataTask.isRunning', function() {
        return !this.get('dataTask.lastSuccessful') && this.get('dataTask.isRunning');
    }),

    totalItems: readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        if (this.get('sort') === 'downloads') {
            return 'All-Time Downloads';
        } else if (this.get('sort') === 'recent-downloads') {
            return 'Recent Downloads';
        } else {
            return 'Relevance';
        }
    }),

    hasItems: bool('totalItems'),

    dataTask: task(function* (params) {
        // debounce the search query
        yield timeout(DEBOUNCE_MS);

        if (params.q !== null) {
            params.q = params.q.trim();
        }

        return yield this.store.query('crate', params);
    }).restartable(),
});
