import { readOnly } from '@ember/object/computed';
import Controller from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'recent-downloads',

    totalItems: readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        if (this.sort === 'downloads') {
            return 'All-Time Downloads';
        } else if (this.sort === 'alpha') {
            return 'Alphabetical';
        } else if (this.get('sort') === 'recent-updates') {
            return 'Recent Updates';
        } else {
            return 'Recent Downloads';
        }
    }),
});
