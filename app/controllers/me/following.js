import { readOnly } from '@ember/object/computed';
import Controller from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../../mixins/pagination';

// TODO: reduce duplicatoin with controllers/me/crates

export default Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'alpha',

    totalItems: readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        return this.sort === 'downloads' ? 'Downloads' : 'Alphabetical';
    }),
});
