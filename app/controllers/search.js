import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias, bool, readOnly } from '@ember/object/computed';
import { inject as service } from '@ember/service';

import PaginationMixin from '../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    search: service(),
    queryParams: ['q', 'page', 'per_page', 'sort'],
    q: alias('search.q'),
    page: '1',
    per_page: 10,
    sort: null,

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
});
