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
        return (this.get('sort') === 'downloads') ? 'Downloads' : 'Relevance';
    }),

    hasItems: computed.bool('totalItems'),
});
