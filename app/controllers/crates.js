import { readOnly } from '@ember/object/computed';
import Controller from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    queryParams: ['letter', 'page', 'per_page', 'sort'],
    letter: null,
    page: '1',
    per_page: 10,
    sort: 'alpha',
    alphabet: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(''),

    totalItems: readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        if (this.sort === 'downloads') {
            return 'All-Time Downloads';
        } else if (this.sort === 'recent-downloads') {
            return 'Recent Downloads';
        } else if (this.get('sort') === 'recent-updates') {
            return 'Recent Updates';
        } else {
            return 'Alphabetical';
        }
    }),

    resultCount: computed('per_page', function() {
        if (this.per_page === 10) {
            return 10;
        } else if (this.per_page === 20) {
            return 20;
        } else if (this.per_page === 50) {
            return 50;
        } else {
            return 100;
        }
    }),
});
