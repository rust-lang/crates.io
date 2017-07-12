import Controller from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    queryParams: ['q', 'page', 'per_page', 'sort'],
    q: null,
    page: '1',
    per_page: 10,
    sort: null,

    totalItems: computed.readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        if (this.get('sort') === 'downloads') {
            return 'All-Time Downloads';
        } else if (this.get('sort') === 'recent-downloads') {
            return 'Recent Downloads';
        } else {
            return 'Relevance';
        }
    }),

    hasItems: computed.bool('totalItems'),
});
