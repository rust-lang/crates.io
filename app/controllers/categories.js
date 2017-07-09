import Controller from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 100,
    sort: 'alpha',

    totalItems: computed.readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        return (this.get('sort') === 'crates') ? '# Crates' : 'Alphabetical';
    }),
});
